use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Instant;

use mago_analyzer::external::ExternalAnalyzer;
use mago_analyzer::external::ExternalAnalyzerError;
use mago_analyzer::external::ExternalAnalyzerHandle;
use mago_extension::WorkerPool;
use mago_linter::external::ExternalLintError;
use mago_linter::external::ExternalLinter;
use mago_orchestrator::OrchestratorError;
use mago_php_version::PHPVersion;

use crate::config::extension::ExtensionHostConfiguration;

/// Starts every enabled extension host, returning one worker pool per host.
///
/// Shared by the linter and analyzer initializers so that a caller wanting both
/// registrations — `mago extension list` does — starts each host once instead of
/// twice.
fn spawn_extension_pools<E>(
    extension_hosts: &BTreeMap<String, ExtensionHostConfiguration>,
    mago_threads: usize,
    kind: &'static str,
    missing_command: impl Fn(String) -> E,
) -> Result<Vec<Arc<WorkerPool>>, E>
where
    E: From<mago_extension::WorkerError>,
{
    extension_hosts
        .iter()
        .filter(|(_, host)| host.enabled)
        .map(|(name, host)| {
            let host_start = tracing::enabled!(tracing::Level::TRACE).then(Instant::now);
            let command = host
                .worker_command()
                .ok_or_else(|| missing_command(format!("enabled extension host `{name}` has no command")))?;

            let size = host.worker_count(mago_threads);
            let options = host.worker_pool_options();
            tracing::trace!(
                host = %name,
                command = ?command,
                workers = size.get(),
                adaptive = host.workers == 0,
                "Starting external {kind} host."
            );

            let pool = if host.workers == 0 {
                WorkerPool::spawn_adaptive(command, size, options)
            } else {
                WorkerPool::spawn(command, size, options)
            };

            let pool = pool.map(Arc::new).map_err(E::from)?;
            if let Some(start) = host_start {
                tracing::trace!(host = %name, active_workers = pool.len(), elapsed = ?start.elapsed(), "External {} host started.", kind);
            }

            Ok::<Arc<WorkerPool>, E>(pool)
        })
        .collect()
}

/// Starts every enabled extension host and validates its linter registration.
pub(crate) fn initialize_external_linter(
    extension_hosts: &BTreeMap<String, ExtensionHostConfiguration>,
    php_version: PHPVersion,
    mago_threads: usize,
) -> Result<Option<ExternalLinter>, ExternalLintError> {
    let trace_start = tracing::enabled!(tracing::Level::TRACE).then(Instant::now);
    tracing::trace!(
        configured_hosts = extension_hosts.len(),
        enabled_hosts = extension_hosts.values().filter(|host| host.enabled).count(),
        mago_threads,
        php_version = %php_version,
        "Initializing external linter hosts."
    );

    let pools = spawn_extension_pools(extension_hosts, mago_threads, "linter", ExternalLintError::Protocol)?;

    if pools.is_empty() {
        tracing::trace!("No external linter hosts are enabled.");
        return Ok(None);
    }

    let linter = ExternalLinter::initialize(pools, php_version)?;
    if let Some(start) = trace_start {
        tracing::trace!(
            extensions = linter.extensions().len(),
            rules = linter.extensions().iter().map(|extension| extension.rules.len()).sum::<usize>(),
            elapsed = ?start.elapsed(),
            "External linter hosts initialized."
        );
    }

    Ok(Some(linter))
}

/// Starts every enabled extension host and validates its analyzer registration.
pub(crate) fn initialize_external_analyzer(
    extension_hosts: &BTreeMap<String, ExtensionHostConfiguration>,
    php_version: PHPVersion,
    mago_threads: usize,
    enabled_plugins: &[String],
    disable_defaults: bool,
) -> Result<Option<ExternalAnalyzer>, ExternalAnalyzerError> {
    let trace_start = tracing::enabled!(tracing::Level::TRACE).then(Instant::now);
    tracing::trace!(
        configured_hosts = extension_hosts.len(),
        enabled_hosts = extension_hosts.values().filter(|host| host.enabled).count(),
        mago_threads,
        php_version = %php_version,
        explicitly_enabled_plugins = enabled_plugins.len(),
        disable_defaults,
        "Initializing external analyzer hosts."
    );

    let pools = spawn_extension_pools(extension_hosts, mago_threads, "analyzer", ExternalAnalyzerError::protocol)?;

    if pools.is_empty() {
        tracing::trace!("No external analyzer hosts are enabled.");
        return Ok(None);
    }

    let analyzer = ExternalAnalyzer::initialize(pools, php_version, enabled_plugins, disable_defaults)?;
    if let Some(start) = trace_start {
        tracing::trace!(
            extensions = analyzer.extensions().len(),
            plugins = analyzer.extensions().iter().map(|extension| extension.plugins.len()).sum::<usize>(),
            elapsed = ?start.elapsed(),
            "External analyzer hosts initialized."
        );
    }

    Ok(Some(analyzer))
}

/// Starts every enabled host once and validates both registrations.
///
/// `mago extension list` and `mago extension validate` want the whole picture —
/// linter rules *and* analyzer plugins — from the same processes, so the pools
/// are spawned once and handed to both initializers.
pub(crate) fn initialize_external_extensions(
    extension_hosts: &BTreeMap<String, ExtensionHostConfiguration>,
    php_version: PHPVersion,
    mago_threads: usize,
    enabled_plugins: &[String],
    disable_defaults: bool,
) -> Result<Option<(ExternalLinter, ExternalAnalyzer)>, OrchestratorError> {
    let pools: Vec<Arc<WorkerPool>> =
        spawn_extension_pools(extension_hosts, mago_threads, "extension", ExternalLintError::Protocol)?;
    if pools.is_empty() {
        tracing::trace!("No external extension hosts are enabled.");
        return Ok(None);
    }

    let linter = ExternalLinter::initialize(pools.clone(), php_version)?;
    let analyzer = ExternalAnalyzer::initialize(pools, php_version, enabled_plugins, disable_defaults)?;

    Ok(Some((linter, analyzer)))
}

/// Starts external analyzer initialization without blocking the codebase pipeline.
pub(crate) fn start_external_analyzer(
    extension_hosts: &BTreeMap<String, ExtensionHostConfiguration>,
    php_version: PHPVersion,
    mago_threads: usize,
    enabled_plugins: &[String],
    disable_defaults: bool,
) -> Option<Arc<ExternalAnalyzerHandle>> {
    if !extension_hosts.values().any(|host| host.enabled) {
        tracing::trace!("Skipping external analyzer initialization because no hosts are enabled.");
        return None;
    }

    let extension_hosts = extension_hosts.clone();
    let enabled_plugins = enabled_plugins.to_vec();
    tracing::trace!(
        enabled_hosts = extension_hosts.values().filter(|host| host.enabled).count(),
        "Starting concurrent external analyzer initialization."
    );

    let initializer = std::thread::Builder::new()
        .name("mago-external-analyzer-init".to_string())
        .spawn(move || {
            let start = tracing::enabled!(tracing::Level::TRACE).then(Instant::now);
            tracing::trace!("External analyzer initialization thread started.");
            let analyzer = initialize_external_analyzer(
                &extension_hosts,
                php_version,
                mago_threads,
                &enabled_plugins,
                disable_defaults,
            )?
            .ok_or_else(|| ExternalAnalyzerError::protocol("external analyzer has no enabled hosts"));
            if let Some(start) = start {
                tracing::trace!(elapsed = ?start.elapsed(), success = analyzer.is_ok(), "External analyzer initialization thread finished.");
            }

            analyzer
        })
        .expect("failed to spawn external analyzer initialization thread");

    Some(Arc::new(ExternalAnalyzerHandle::pending(initializer)))
}
