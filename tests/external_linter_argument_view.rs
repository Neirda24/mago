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

use Acme\Security\IsGranted;

final class Controller
{
    #[IsGranted('ROLE_HR', subject: 'invoice', message: 'You need it')]
    public function promote(): void {}

    #[IsGranted(attribute: 'ROLE_ADMIN')]
    public function demote(): void {}

    #[Bare, Second('only')]
    public function bare(): void {}

    public function report(array $rest): void
    {
        describe('first', 'second', subject: 'named');
        spread(...$rest);
    }
}
"#;

/// Drives the real linter so the whole path — Rust snapshot, protocol, PHP
/// views — is under test.
#[test]
fn external_linter_reads_attribute_and_call_arguments() -> Result<(), Box<dyn std::error::Error>> {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    if !common::php_sdk_is_available(repository, "the external linter argument-view test") {
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
            // One `#[...]` group of two: `fromList()` reads both, and `#[Bare]`
            // without parentheses is not `#[Bare()]`.
            "attribute name=Bare resolved=App\\Bare parens=no count=0 [] first=- attribute=-".to_owned(),
            // Written by name only: there is no first *positional* argument,
            // and asking for `attribute` finds the value.
            "attribute name=IsGranted resolved=Acme\\Security\\IsGranted parens=yes count=1 \
             [0:attribute='ROLE_ADMIN'] first=- attribute='ROLE_ADMIN'"
                .to_owned(),
            // `argument(0)` is the positional one; source order is on `$index`.
            "attribute name=IsGranted resolved=Acme\\Security\\IsGranted parens=yes count=3 \
             [0:0='ROLE_HR' 1:subject='invoice' 2:message='You need it'] first='ROLE_HR' attribute=-"
                .to_owned(),
            "attribute name=Second resolved=App\\Second parens=yes count=1 [0:0='only'] \
             first='only' attribute=-"
                .to_owned(),
            // The same selectors on a call, where index and position diverge.
            "call name=describe count=3 [0:0='first' 1:1='second' 2:subject='named'] first='first' \
             second='second' subject='named'"
                .to_owned(),
            // An unpacked argument stays positional and is flagged.
            "call name=spread count=1 [0:0...=$rest] first=$rest second=- subject=-".to_owned(),
        ]
    );

    Ok(())
}
