<?php

declare(strict_types=1);

namespace Mago\Sdk\Syntax;

use Mago\Sdk\Exception\InvalidArgumentException;
use Mago\Sdk\Internal\Syntax\NodeReader;

/**
 * A structured view over a single attribute.
 *
 * An attribute is a constructor call and reads like one. It needs its own entry
 * point only because it carries a `PartialArgumentList` where a call carries an
 * `ArgumentList`, which is why `CallExpression::fromNode()` rejects it.
 *
 * @api
 * @mago-expect lint:cyclomatic-complexity
 */
final class AttributeExpression
{
    /** @var list<CallArgument> */
    public readonly array $arguments;

    /**
     * @param list<CallArgument> $arguments
     */
    private function __construct(
        public readonly Node $node,
        public readonly Node $name,
        public readonly ?Node $argumentList,
        array $arguments,
    ) {
        $this->arguments = $arguments;
    }

    public static function fromNode(SourceFile $source, Node $node): self
    {
        if ($node->kind !== NodeKind::Attribute) {
            throw new InvalidArgumentException('An attribute view requires an attribute node.');
        }

        $children = $source->getChildren($node);
        $name = $children[0] ?? throw new InvalidArgumentException('An attribute node has no name.');
        $argumentList = $children[1] ?? null;
        if ($argumentList !== null && $argumentList->kind !== NodeKind::PartialArgumentList) {
            throw new InvalidArgumentException('An attribute node has an unexpected argument list.');
        }

        return new self(
            $node,
            $name,
            $argumentList,
            $argumentList === null ? [] : NodeReader::readArgumentList($source, $argumentList),
        );
    }

    /**
     * Every attribute written in one `AttributeList` node, in source order:
     * `#[Foo, Bar(1)]` is one list of two.
     *
     * @return list<self>
     */
    public static function fromList(SourceFile $source, Node $node): array
    {
        if ($node->kind !== NodeKind::AttributeList) {
            throw new InvalidArgumentException('An attribute list view requires an attribute-list node.');
        }

        $attributes = [];
        foreach ($source->getChildren($node) as $attribute) {
            $attributes[] = self::fromNode($source, $attribute);
        }

        return $attributes;
    }

    /**
     * The name as written, which may be unqualified, qualified, or fully qualified.
     */
    public function getName(SourceFile $source): string
    {
        return $source->getText($this->name);
    }

    /**
     * The name as resolved against the file's `use` statements and namespace.
     */
    public function getResolvedName(SourceFile $source): ?ResolvedName
    {
        return $source->getResolvedName($this->name);
    }

    /**
     * Whether the attribute was written with parentheses.
     *
     * `#[Foo]` has no argument list; `#[Foo()]` has an empty one.
     */
    public function hasArgumentList(): bool
    {
        return $this->argumentList !== null;
    }

    /**
     * Selects one argument by positional position, or by name. An integer
     * selector counts positional arguments only; read `CallArgument::$index`
     * for the source-order index instead.
     *
     * @param non-negative-int|string $selector
     */
    public function argument(int|string $selector): ?CallArgument
    {
        return NodeReader::selectArgument($this->arguments, $selector);
    }
}
