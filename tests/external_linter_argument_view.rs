#![allow(clippy::expect_used, clippy::missing_panics_doc, clippy::unwrap_used)]

mod common;

use std::borrow::Cow;
use std::num::NonZeroUsize;
use std::path::Path;
use std::sync::Arc;

use mago_allocator::LocalArena;
use mago_database::file::File;
use mago_extension::WorkerCommand;
use mago_extension::WorkerPool;
use mago_extension::WorkerPoolOptions;
use mago_linter::Linter;
use mago_linter::external::ExternalLinter;
use mago_linter::settings::Settings;
use mago_names::resolver::NameResolver;
use mago_php_version::PHPVersion;
use mago_syntax::parser::parse_file;

const SOURCE: &[u8] = br#"<?php

namespace App;

function report(array $rest): void
{
    describe('first', 'second', subject: 'named');
    spread(...$rest);
}
"#;

/// Drives the real linter so the whole path — Rust snapshot, protocol, PHP
/// view — is under test.
#[test]
fn external_linter_reads_call_arguments() -> Result<(), Box<dyn std::error::Error>> {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    if !common::php_sdk_is_available(repository, "the external linter call-argument test") {
        return Ok(());
    }

    let command = WorkerCommand::new("php")
        .with_argument(repository.join("composer/tests/Sdk/worker.php"))
        .with_current_directory(repository);
    let pool = WorkerPool::spawn(command, NonZeroUsize::MIN, WorkerPoolOptions::default())?;
    let external = ExternalLinter::initialize([Arc::new(pool)], PHPVersion::PHP85)?;

    let file = File::ephemeral(Cow::Borrowed(b"src/Controller.php"), Cow::Borrowed(SOURCE));
    let arena = LocalArena::new();
    let program = parse_file(&arena, &file);
    let resolved_names = NameResolver::new(&arena).resolve(program);
    let settings = Settings { php_version: PHPVersion::PHP85, ..Settings::default() };
    let only = vec!["mago-sdk-test/argument-view".to_owned()];
    let linter = Linter::new(&arena, &settings, Some(&only), false);

    let issues = linter.lint_with_external(&file, program, &resolved_names, &external)?;
    let mut messages = issues
        .iter()
        .filter(|issue| issue.code.as_deref() == Some("mago-sdk-test/argument-view"))
        .map(|issue| issue.message.clone())
        .collect::<Vec<_>>();
    messages.sort();

    assert_eq!(
        messages,
        vec![
            // `argument(0)` and `argument(1)` count positional arguments only, so
            // the named one in between shifts `$index` but not the position.
            "call name=describe count=3 [0:0='first' 1:1='second' 2:subject='named'] first='first' \
             second='second' subject='named'"
                .to_owned(),
            // An unpacked argument stays positional and is flagged.
            "call name=spread count=1 [0:0...=$rest] first=$rest second=- subject=-".to_owned(),
        ]
    );

    Ok(())
}
