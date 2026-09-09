//! Inspect and validate configured external extension hosts.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::process::ExitCode;

use clap::Parser;
use clap::Subcommand;

use mago_analyzer::external::ExternalPlugin;
use mago_linter::external::ExternalRule;

use crate::config::Configuration;
use crate::config::extension::ExtensionHostConfiguration;
use crate::error::Error;
use crate::extensions::initialize_external_extensions;

/// Manage external extensions configured for this workspace.
#[derive(Parser, Debug)]
#[command(name = "extension", about = "Inspect and validate external extensions.")]
pub struct ExtensionCommand {
    #[command(subcommand)]
    command: ExtensionSubcommand,
}

#[derive(Subcommand, Debug)]
enum ExtensionSubcommand {
    /// List configured extensions, the linter rules and the analyzer plugins they expose.
    List {
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Start every configured host and validate its registration.
    Validate,
}

/// One logical extension, with everything it registers.
///
/// An extension advertises linter rules and analyzer plugins through two
/// separate registrations, so the two are joined by identifier here. An
/// extension that registers only analyzer plugins is a real case and used to be
/// invisible to this command entirely.
#[derive(Debug, Default)]
struct ListedExtension<'registration> {
    name: &'registration str,
    version: &'registration str,
    rules: &'registration [ExternalRule],
    plugins: &'registration [ExternalPlugin],
}

impl ExtensionCommand {
    pub fn execute(self, configuration: Configuration) -> Result<ExitCode, Error> {
        let mago_threads = configuration.threads;
        let enabled_hosts = configuration.extension_hosts.iter().filter(|(_, host)| host.enabled).collect::<Vec<_>>();
        let enabled_plugins = &configuration.analyzer.plugins;
        let disable_defaults = configuration.analyzer.disable_default_plugins;
        let external = initialize_external_extensions(
            &configuration.extension_hosts,
            configuration.php_version,
            mago_threads,
            enabled_plugins,
            disable_defaults,
        )?;

        match self.command {
            ExtensionSubcommand::List { json } => {
                let Some((linter, analyzer)) = external.as_ref() else {
                    if json {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "hosts": Vec::<serde_json::Value>::new(),
                                "extensions": Vec::<serde_json::Value>::new(),
                            }))?
                        );
                    } else {
                        println!("No external extensions are configured.");
                    }

                    return Ok(ExitCode::SUCCESS);
                };

                let listed = join_registrations(linter.extensions(), analyzer.extensions());
                if json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&list_json(
                            &enabled_hosts,
                            mago_threads,
                            &listed,
                            enabled_plugins,
                            disable_defaults,
                        ))?
                    );
                } else {
                    print!("{}", render_list(&enabled_hosts, mago_threads, &listed, enabled_plugins, disable_defaults));
                }
            }
            ExtensionSubcommand::Validate => {
                if let Some((linter, analyzer)) = external.as_ref() {
                    let listed = join_registrations(linter.extensions(), analyzer.extensions());
                    let rules = listed.values().map(|extension| extension.rules.len()).sum::<usize>();
                    let plugins = listed.values().map(|extension| extension.plugins.len()).sum::<usize>();
                    println!(
                        "Validated {} extension(s) from {} host(s): {rules} linter rule(s), {plugins} analyzer plugin(s).",
                        listed.len(),
                        enabled_hosts.len(),
                    );
                } else {
                    println!("No external extensions are configured.");
                }
            }
        }

        Ok(ExitCode::SUCCESS)
    }
}

/// Joins the linter and analyzer registrations by extension identifier.
fn join_registrations<'registration>(
    linter: &'registration [mago_linter::external::ExternalExtension],
    analyzer: &'registration [mago_analyzer::external::ExternalExtension],
) -> BTreeMap<&'registration str, ListedExtension<'registration>> {
    let mut listed: BTreeMap<&str, ListedExtension<'_>> = BTreeMap::new();
    for extension in linter {
        let entry = listed.entry(extension.identifier.as_str()).or_default();
        entry.name = extension.name.as_str();
        entry.version = extension.version.as_str();
        entry.rules = &extension.rules;
    }

    for extension in analyzer {
        let entry = listed.entry(extension.identifier.as_str()).or_default();
        entry.name = extension.name.as_str();
        entry.version = extension.version.as_str();
        entry.plugins = &extension.plugins;
    }

    listed
}

fn render_list(
    enabled_hosts: &[(&String, &ExtensionHostConfiguration)],
    mago_threads: usize,
    listed: &BTreeMap<&str, ListedExtension<'_>>,
    enabled_plugins: &[String],
    disable_defaults: bool,
) -> String {
    let mut output = String::new();
    output.push_str("Extension hosts:\n");
    for (host, host_configuration) in enabled_hosts {
        let workers = host_configuration.worker_count(mago_threads);
        if host_configuration.workers == 0 {
            let _ = writeln!(output, "  {host} (adaptive, up to {workers} workers)");
        } else {
            let _ = writeln!(output, "  {host} ({workers} workers)");
        }
    }

    output.push_str("Registered extensions:\n");
    for (identifier, extension) in listed {
        let _ = writeln!(output, "{} ({identifier})", extension.name);
        let _ = writeln!(output, "  Version: {}", extension.version);
        let _ = writeln!(output, "  Linter rules: {}", extension.rules.len());
        for rule in extension.rules {
            let _ = writeln!(output, "    {} ({})", rule.code, rule.default_level);
        }

        let _ = writeln!(output, "  Analyzer plugins: {}", extension.plugins.len());
        for plugin in extension.plugins {
            let state = if plugin.is_enabled_by(enabled_plugins, disable_defaults) {
                "enabled"
            } else {
                "not enabled; add it to analyzer.plugins"
            };

            let hooks = plugin.hooks();
            let _ = writeln!(
                output,
                "    {} ({state}) — {} hook(s){}{}",
                plugin.identifier,
                hooks.len(),
                if hooks.is_empty() { "" } else { ": " },
                hooks.join(", "),
            );
        }
    }

    output
}

fn list_json(
    enabled_hosts: &[(&String, &ExtensionHostConfiguration)],
    mago_threads: usize,
    listed: &BTreeMap<&str, ListedExtension<'_>>,
    enabled_plugins: &[String],
    disable_defaults: bool,
) -> serde_json::Value {
    let hosts = enabled_hosts
        .iter()
        .map(|(host, host_configuration)| {
            serde_json::json!({
                "host": host,
                "adaptive": host_configuration.workers == 0,
                "workers": host_configuration.worker_count(mago_threads).get(),
            })
        })
        .collect::<Vec<_>>();

    let extensions = listed
        .iter()
        .map(|(identifier, extension)| {
            serde_json::json!({
                "identifier": identifier,
                "name": extension.name,
                "version": extension.version,
                "linter-rules": extension.rules.iter().map(|rule| serde_json::json!({
                    "code": rule.code,
                    "name": rule.name,
                    "description": rule.description,
                    "default-level": rule.default_level,
                    "default-enabled": rule.default_enabled,
                    "targets": rule.targets.iter().map(ToString::to_string).collect::<Vec<_>>(),
                })).collect::<Vec<_>>(),
                "analyzer-plugins": extension.plugins.iter().map(|plugin| serde_json::json!({
                    "identifier": plugin.identifier,
                    "name": plugin.name,
                    "description": plugin.description,
                    "aliases": plugin.aliases,
                    "default-enabled": plugin.default_enabled,
                    "enabled": plugin.is_enabled_by(enabled_plugins, disable_defaults),
                    "hooks": plugin.hooks(),
                })).collect::<Vec<_>>(),
            })
        })
        .collect::<Vec<_>>();

    serde_json::json!({ "hosts": hosts, "extensions": extensions })
}

#[cfg(test)]
mod tests {
    use mago_reporting::Level;
    use mago_syntax::cst::NodeKind;

    use super::*;

    fn plugin(identifier: &str, default_enabled: bool, node_analysis: bool, after_analysis: bool) -> ExternalPlugin {
        ExternalPlugin {
            index: 0,
            extension: "acme/conventions".to_owned(),
            identifier: identifier.to_owned(),
            name: identifier.to_owned(),
            description: "A plugin.".to_owned(),
            aliases: Vec::new(),
            default_enabled,
            initialization: false,
            before_analysis: false,
            after_file_analysis: false,
            after_file_expression_types: false,
            node_analysis,
            after_analysis,
        }
    }

    fn rule(code: &str) -> ExternalRule {
        ExternalRule {
            code: code.to_owned(),
            name: code.to_owned(),
            description: "A rule.".to_owned(),
            default_level: Level::Error,
            default_enabled: true,
            targets: vec![NodeKind::Class],
        }
    }

    #[test]
    fn joins_an_extension_registering_only_analyzer_plugins() {
        let linter = vec![mago_linter::external::ExternalExtension {
            identifier: "acme/rules".to_owned(),
            name: "Acme Rules".to_owned(),
            version: "1.0.0".to_owned(),
            rules: vec![rule("acme/one")],
        }];
        let analyzer = vec![mago_analyzer::external::ExternalExtension {
            identifier: "acme/plugins".to_owned(),
            name: "Acme Plugins".to_owned(),
            version: "2.0.0".to_owned(),
            plugins: vec![plugin("acme/doctrine", true, true, false)],
        }];

        let listed = join_registrations(&linter, &analyzer);

        // Before the join, an extension advertising analyzer plugins and no
        // linter rules did not appear in the listing at all.
        assert_eq!(listed.len(), 2);
        assert_eq!(listed["acme/plugins"].plugins.len(), 1);
        assert_eq!(listed["acme/plugins"].version, "2.0.0");
        assert!(listed["acme/plugins"].rules.is_empty());
        assert!(listed["acme/rules"].plugins.is_empty());
    }

    #[test]
    fn renders_plugins_with_their_hooks_and_whether_they_run() {
        let analyzer = vec![mago_analyzer::external::ExternalExtension {
            identifier: "acme/conventions".to_owned(),
            name: "Conventions".to_owned(),
            version: "1.2.0".to_owned(),
            plugins: vec![
                plugin("acme/doctrine", true, true, true),
                plugin("acme/symfony-security", false, true, false),
            ],
        }];
        let listed = join_registrations(&[], &analyzer);

        let output = render_list(&[], 4, &listed, &[], false);

        assert!(output.contains("  Analyzer plugins: 2\n"), "{output}");
        assert!(
            output.contains("    acme/doctrine (enabled) — 2 hook(s): node-analysis, after-analysis\n"),
            "{output}"
        );
        // The most common extension misconfiguration: registered, validated, and
        // never invoked because `analyzer.plugins` does not select it.
        assert!(
            output.contains(
                "    acme/symfony-security (not enabled; add it to analyzer.plugins) — 1 hook(s): node-analysis\n"
            ),
            "{output}"
        );
    }

    #[test]
    fn a_selected_plugin_is_reported_as_enabled() {
        let analyzer = vec![mago_analyzer::external::ExternalExtension {
            identifier: "acme/conventions".to_owned(),
            name: "Conventions".to_owned(),
            version: "1.2.0".to_owned(),
            plugins: vec![plugin("acme/symfony-security", false, true, false)],
        }];
        let listed = join_registrations(&[], &analyzer);
        let selected = vec!["acme/symfony-security".to_owned()];

        let output = render_list(&[], 4, &listed, &selected, false);
        assert!(output.contains("    acme/symfony-security (enabled)"), "{output}");

        let json = list_json(&[], 4, &listed, &selected, false);
        assert_eq!(json["extensions"][0]["analyzer-plugins"][0]["enabled"], serde_json::json!(true));
        assert_eq!(json["extensions"][0]["analyzer-plugins"][0]["hooks"], serde_json::json!(["node-analysis"]));
    }

    #[test]
    fn disabling_defaults_turns_a_default_plugin_off() {
        let analyzer = vec![mago_analyzer::external::ExternalExtension {
            identifier: "acme/conventions".to_owned(),
            name: "Conventions".to_owned(),
            version: "1.2.0".to_owned(),
            plugins: vec![plugin("acme/doctrine", true, true, false)],
        }];
        let listed = join_registrations(&[], &analyzer);

        let json = list_json(&[], 4, &listed, &[], true);
        assert_eq!(json["extensions"][0]["analyzer-plugins"][0]["enabled"], serde_json::json!(false));
    }
}
