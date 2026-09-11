<?php

declare(strict_types=1);

namespace Mago\Tests\Sdk\Unit\Syntax;

use Mago\Sdk\Exception\InvalidArgumentException;
use Mago\Sdk\Internal\Syntax\NodeStore;
use Mago\Sdk\Internal\Syntax\ResolvedNameStore;
use Mago\Sdk\Internal\Syntax\ScopeStore;
use Mago\Sdk\Internal\Syntax\TriviaStore;
use Mago\Sdk\PHPVersion;
use Mago\Sdk\Syntax\AttributeExpression;
use Mago\Sdk\Syntax\NodeKind;
use Mago\Sdk\Syntax\SourceFile;
use PHPUnit\Framework\TestCase;

use function pack;

final class AttributeExpressionTest extends TestCase
{
    public function testFromNodeRejectsANodeThatIsNotAnAttribute(): void
    {
        $source = self::singleNode(NodeKind::Program);

        $this->expectException(InvalidArgumentException::class);
        $this->expectExceptionMessage('An attribute view requires an attribute node.');

        AttributeExpression::fromNode($source, $source->getNode(0));
    }

    public function testFromListRejectsANodeThatIsNotAnAttributeList(): void
    {
        $source = self::singleNode(NodeKind::Attribute);

        $this->expectException(InvalidArgumentException::class);
        $this->expectExceptionMessage('An attribute list view requires an attribute-list node.');

        AttributeExpression::fromList($source, $source->getNode(0));
    }

    public function testAnAttributeWithoutChildrenIsRejected(): void
    {
        $source = self::singleNode(NodeKind::Attribute);

        $this->expectException(InvalidArgumentException::class);
        $this->expectExceptionMessage('An attribute node has no name.');

        AttributeExpression::fromNode($source, $source->getNode(0));
    }

    /**
     * A snapshot holding one childless node of the given kind.
     */
    private static function singleNode(NodeKind $kind): SourceFile
    {
        $noNode = 4_294_967_295;

        return new SourceFile(
            PHPVersion::fromParts(8, 1),
            'fixture.php',
            '#[Foo]',
            [0],
            new NodeStore([$kind], pack('CNNNNN', 0, 0, 6, $noNode, $noNode, $noNode), 1),
            new ResolvedNameStore('', '', '', 0),
            new TriviaStore('', 0),
            null,
            new ScopeStore(pack('N2', 0, 0), '', 1),
        );
    }
}
