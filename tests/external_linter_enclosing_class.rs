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

namespace App\Controller;

final class InvoiceController
{
    public function save(): void
    {
        $this->addFlash('success', 'Saved.');
    }
}

trait Loggable
{
    public function log(): void
    {
        $this->logger->info('written');
    }
}

function helper(object $subject): void
{
    $subject->touch();
}

$anonymous = new class {
    public function run(): void
    {
        $this->go();
    }
};
"#;

/// A rule targeting a call gets that call's subtree and nothing above it, so it
/// cannot walk up to the class holding the call. The enclosing class's resolved
/// name travels beside the target instead.
#[test]
fn external_linter_reports_the_enclosing_class() -> Result<(), Box<dyn std::error::Error>> {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    if !common::php_sdk_is_available(repository, "the external linter enclosing-class test") {
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
    let only = vec!["mago-sdk-test/enclosing-class".to_owned()];
    let linter = Linter::new(&arena, &settings, Some(&only), false);

    let issues = linter.lint_with_external(&file, program, &resolved_names, &external)?;
    let mut messages = issues
        .iter()
        .filter(|issue| issue.code.as_deref() == Some("mago-sdk-test/enclosing-class"))
        .map(|issue| issue.message.clone())
        .collect::<Vec<_>>();
    messages.sort();

    assert_eq!(
        messages,
        vec![
            // A method call in a class reports that class, fully qualified,
            // while `getAncestors()` still answers nothing: the snapshot holds
            // the target's subtree only, which is what makes the name necessary.
            "call=addFlash class=App\\Controller\\InvoiceController ancestors=0".to_owned(),
            // Inside an anonymous class: a class-like boundary with no name, and
            // not the same thing as being at file level's enclosing class.
            "call=go class=- ancestors=0".to_owned(),
            // A trait is a class-like too.
            "call=info class=App\\Controller\\Loggable ancestors=0".to_owned(),
            // A plain function has no enclosing class, and the rule still fires
            // — the point of not making rules target the class instead.
            "call=touch class=- ancestors=0".to_owned(),
        ]
    );

    Ok(())
}
