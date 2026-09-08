<?php

declare(strict_types=1);

namespace Mago\Tests\Sdk\Fixture;

use Mago\Sdk\Linter\LintContext;
use Mago\Sdk\Linter\Rule;
use Mago\Sdk\Linter\RuleDefinition;
use Mago\Sdk\Reporting\Issue;
use Mago\Sdk\Reporting\Level;
use Mago\Sdk\Syntax\CallExpression;
use Mago\Sdk\Syntax\NodeKind;

use function count;
use function sprintf;

/**
 * Reports which class each call is inside, so a test can assert it.
 */
final class EnclosingClassRule implements Rule
{
    public function getDefinition(): RuleDefinition
    {
        return new RuleDefinition(
            code: 'mago-sdk-test/enclosing-class',
            name: 'Enclosing class',
            description: 'Reports the class enclosing every method call.',
            defaultLevel: Level::Help,
            defaultEnabled: true,
            targets: [NodeKind::MethodCall],
        );
    }

    public function lint(LintContext $context): void
    {
        $call = CallExpression::fromNode($context->file, $context->node);

        $context->report(Issue::new(
            sprintf(
                'call=%s class=%s ancestors=%d',
                $call->getName($context->file) ?? '-',
                $context->getEnclosingClassName() ?? '-',
                count($context->file->getAncestors($context->node)),
            ),
            $context->node->span,
        ));
    }
}
