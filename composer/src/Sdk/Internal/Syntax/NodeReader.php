<?php

declare(strict_types=1);

namespace Mago\Sdk\Internal\Syntax;

use Mago\Sdk\Syntax\CallArgument;
use Mago\Sdk\Syntax\Node;
use Mago\Sdk\Syntax\NodeKind;
use Mago\Sdk\Syntax\SourceFile;

use function count;
use function is_int;
use function str_starts_with;

/**
 * Low-level reads shared by the structured views in `Mago\Sdk\Syntax`.
 *
 * @internal
 * @mago-expect lint:cyclomatic-complexity
 */
final class NodeReader
{
    /**
     * Reads an `ArgumentList` or a `PartialArgumentList` into argument views, in
     * source order. Placeholder variants (`?`, `...`, `name: ?`) hold no value
     * and are skipped.
     *
     * @return list<CallArgument>
     */
    public static function readArgumentList(SourceFile $source, Node $argumentList): array
    {
        $arguments = [];
        $position = 0;
        foreach ($source->getChildren($argumentList) as $argument) {
            $variant = $source->getChildren($argument)[0] ?? null;
            if ($variant === null) {
                continue;
            }

            $named = match ($variant->kind) {
                NodeKind::NamedArgument => true,
                NodeKind::PositionalArgument => false,
                default => null,
            };

            if ($named === null) {
                continue;
            }

            $parts = $source->getChildren($variant);
            $value = $parts[$named ? 1 : 0] ?? null;
            if ($value === null) {
                continue;
            }

            $arguments[] = new CallArgument(
                count($arguments),
                $argument,
                self::unwrapExpression($source, $value),
                $named ? $source->getText($parts[0]) : null,
                // `name: ...$value` is not valid PHP, so only a positional argument unpacks.
                !$named && str_starts_with($source->getText($variant), '...'),
                $named ? null : $position++,
            );
        }

        return $arguments;
    }

    /**
     * Selects one argument by positional position or by name.
     *
     * @param list<CallArgument> $arguments
     * @param non-negative-int|string $selector
     */
    public static function selectArgument(array $arguments, int|string $selector): ?CallArgument
    {
        foreach ($arguments as $argument) {
            $matches = is_int($selector) ? $argument->position === $selector : $argument->name === $selector;
            if ($matches) {
                return $argument;
            }
        }

        return null;
    }

    /** Descends through `Expression` wrappers to the node they hold. */
    public static function unwrapExpression(SourceFile $source, Node $node): Node
    {
        while ($node->kind === NodeKind::Expression) {
            $next = $source->getChildren($node)[0] ?? null;
            if ($next === null) {
                break;
            }

            $node = $next;
        }

        return $node;
    }
}
