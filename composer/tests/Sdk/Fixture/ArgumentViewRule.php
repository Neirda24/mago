<?php

declare(strict_types=1);

namespace Mago\Tests\Sdk\Fixture;

use Mago\Sdk\Linter\LintContext;
use Mago\Sdk\Linter\Rule;
use Mago\Sdk\Linter\RuleDefinition;
use Mago\Sdk\Reporting\Issue;
use Mago\Sdk\Reporting\Level;
use Mago\Sdk\Syntax\AttributeExpression;
use Mago\Sdk\Syntax\CallArgument;
use Mago\Sdk\Syntax\CallExpression;
use Mago\Sdk\Syntax\NodeKind;

use function count;
use function implode;
use function sprintf;

/**
 * Reports what the SDK's argument views see, so a test can assert it. One rule
 * subscribes to both kinds, since both are read through the same model.
 *
 * @mago-expect lint:cyclomatic-complexity
 */
final class ArgumentViewRule implements Rule
{
    public function getDefinition(): RuleDefinition
    {
        return new RuleDefinition(
            code: 'mago-sdk-test/argument-view',
            name: 'Argument view',
            description: 'Reports the argument view of every attribute and function call.',
            defaultLevel: Level::Help,
            defaultEnabled: true,
            targets: [NodeKind::AttributeList, NodeKind::FunctionCall],
        );
    }

    public function lint(LintContext $context): void
    {
        if ($context->node->kind === NodeKind::FunctionCall) {
            $call = CallExpression::fromNode($context->file, $context->node);

            $context->report(Issue::new(
                sprintf(
                    'call name=%s %s first=%s second=%s subject=%s',
                    $call->getName($context->file) ?? '-',
                    self::describe($context, $call->arguments),
                    self::text($context, $call->argument(0)),
                    self::text($context, $call->argument(1)),
                    self::text($context, $call->argument('subject')),
                ),
                $context->node->span,
            ));

            return;
        }

        foreach (AttributeExpression::fromList($context->file, $context->node) as $attribute) {
            $context->report(Issue::new(
                sprintf(
                    'attribute name=%s resolved=%s parens=%s %s first=%s attribute=%s',
                    $attribute->getName($context->file),
                    $attribute->getResolvedName($context->file)->name ?? '-',
                    $attribute->hasArgumentList() ? 'yes' : 'no',
                    self::describe($context, $attribute->arguments),
                    self::text($context, $attribute->argument(0)),
                    self::text($context, $attribute->argument('attribute')),
                ),
                $attribute->node->span,
            ));
        }
    }

    /**
     * @param list<CallArgument> $arguments
     */
    private static function describe(LintContext $context, array $arguments): string
    {
        $written = [];
        foreach ($arguments as $argument) {
            $written[] = sprintf(
                '%d:%s%s=%s',
                $argument->index,
                $argument->position === null ? $argument->name ?? '?' : (string) $argument->position,
                $argument->unpacked ? '...' : '',
                $context->file->getText($argument->value),
            );
        }

        return sprintf('count=%d [%s]', count($arguments), implode(' ', $written));
    }

    private static function text(LintContext $context, ?CallArgument $argument): string
    {
        return $argument === null ? '-' : $context->file->getText($argument->value);
    }
}
