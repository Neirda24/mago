<?php

declare(strict_types=1);

namespace Mago\Sdk\Syntax;

use Mago\Sdk\Exception\InvalidArgumentException;
use Mago\Sdk\Internal\Syntax\NodeReader;

/**
 * A structured view over a function, method, or static-method call.
 *
 * @api
 * @mago-expect lint:cyclomatic-complexity
 */
final class CallExpression
{
    /** @var list<CallArgument> */
    public readonly array $arguments;

    /**
     * @param list<CallArgument> $arguments
     */
    private function __construct(
        public readonly Node $node,
        public readonly Node $callee,
        public readonly ?Node $receiver,
        public readonly ?Node $member,
        array $arguments,
    ) {
        $this->arguments = $arguments;
    }

    public static function fromNode(SourceFile $source, Node $node): self
    {
        if (
            $node->kind !== NodeKind::FunctionCall
            && $node->kind !== NodeKind::MethodCall
            && $node->kind !== NodeKind::NullSafeMethodCall
            && $node->kind !== NodeKind::StaticMethodCall
        ) {
            throw new InvalidArgumentException('A call-expression view requires a call node.');
        }

        $children = $source->getChildren($node);
        $function = $node->kind === NodeKind::FunctionCall;
        $callee = NodeReader::unwrapExpression(
            $source,
            $children[0] ?? throw new InvalidArgumentException('A call node has no callee.'),
        );
        $member = $function ? null : $children[1] ?? null;
        $argumentList = $children[$function ? 1 : 2] ?? null;
        if ($argumentList === null || $argumentList->kind !== NodeKind::ArgumentList) {
            throw new InvalidArgumentException('A call node has no argument list.');
        }

        return new self(
            $node,
            $callee,
            $function ? null : $callee,
            $member,
            NodeReader::readArgumentList($source, $argumentList),
        );
    }

    public static function fromExpression(SourceFile $source, Node $node): ?self
    {
        $node = NodeReader::unwrapExpression($source, $node);
        while ($node->kind === NodeKind::Call) {
            $next = $source->getChildren($node)[0] ?? null;
            if ($next === null) {
                break;
            }
            $node = NodeReader::unwrapExpression($source, $next);
        }

        return match ($node->kind) {
            NodeKind::FunctionCall,
            NodeKind::MethodCall,
            NodeKind::NullSafeMethodCall,
            NodeKind::StaticMethodCall,
                => self::fromNode($source, $node),
            default => null,
        };
    }

    public function isFunction(): bool
    {
        return $this->node->kind === NodeKind::FunctionCall;
    }

    public function isStaticMethod(): bool
    {
        return $this->node->kind === NodeKind::StaticMethodCall;
    }

    public function isMethod(): bool
    {
        return $this->node->kind === NodeKind::MethodCall || $this->node->kind === NodeKind::NullSafeMethodCall;
    }

    /**
     * Selects one argument by positional position, or by name. `argument(1)` on
     * `f($a, b: $b, $c)` is `$c`; read `CallArgument::$index` for source order.
     *
     * @param non-negative-int|string $selector
     */
    public function argument(int|string $selector): ?CallArgument
    {
        return NodeReader::selectArgument($this->arguments, $selector);
    }

    public function getName(SourceFile $source): ?string
    {
        if ($this->isFunction()) {
            $callee = $this->callee;
            while ($callee->kind === NodeKind::ConstantAccess || $callee->kind === NodeKind::Expression) {
                $next = $source->getChildren($callee)[0] ?? null;
                if ($next === null) {
                    break;
                }
                $callee = $next;
            }

            return match ($callee->kind) {
                NodeKind::Identifier,
                NodeKind::LocalIdentifier,
                NodeKind::QualifiedIdentifier,
                NodeKind::FullyQualifiedIdentifier,
                    => $source->getText($callee),
                default => null,
            };
        }

        if ($this->member === null) {
            return null;
        }
        $selector = $source->getChildren($this->member)[0] ?? null;

        if ($selector === null || $selector->kind !== NodeKind::LocalIdentifier) {
            return null;
        }

        return $source->getText($selector);
    }
}
