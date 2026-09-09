<?php

declare(strict_types=1);

namespace Mago\Sdk\Syntax;

/**
 * One argument in a Mago call-expression view.
 *
 * @api
 */
final class CallArgument
{
    /**
     * @param non-negative-int $index Source-order index in the containing argument list, named arguments included.
     * @param non-negative-int|null $position Index among the positional arguments only, `null` when named.
     *                                        `f($a, b: $b, $c)` gives `$c` index 2 and position 1.
     * @mago-expect lint:excessive-parameter-list
     */
    public function __construct(
        public readonly int $index,
        public readonly Node $node,
        public readonly Node $value,
        public readonly ?string $name,
        public readonly bool $unpacked,
        public readonly ?int $position = null,
    ) {}
}
