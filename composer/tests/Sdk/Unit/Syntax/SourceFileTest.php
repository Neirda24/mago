<?php

declare(strict_types=1);

namespace Mago\Tests\Sdk\Unit\Syntax;

use Mago\Sdk\Internal\Syntax\LiteralStringStore;
use Mago\Sdk\Internal\Syntax\NodeStore;
use Mago\Sdk\Internal\Syntax\ResolvedNameStore;
use Mago\Sdk\Internal\Syntax\ScopeStore;
use Mago\Sdk\Internal\Syntax\TriviaStore;
use Mago\Sdk\PHPVersion;
use Mago\Sdk\Syntax\NodeKind;
use Mago\Sdk\Syntax\SourceFile;
use Mago\Sdk\Syntax\TriviaKind;
use PHPUnit\Framework\TestCase;

use function pack;
use function strlen;

final class SourceFileTest extends TestCase
{
    public function testPackedSourceDataIsExposed(): void
    {
        $noNode = 4_294_967_295;
        $nodeRecords =
            pack('CNNNNN', 0, 0, 10, $noNode, 1, $noNode)
            . pack('CNNNNN', 1, 1, 4, 0, $noNode, 2)
            . pack('CNNNNN', 1, 5, 8, 0, $noNode, $noNode);
        $nodeStore = new NodeStore([NodeKind::Program, NodeKind::FunctionCall], $nodeRecords, 3);
        $resolvedName = 'Psl\\Iter\\any';
        $nameStarts = pack('N', 1);
        $nameRecords = pack('NNNC', 4, 0, strlen($resolvedName), 0);
        $nameStore = new ResolvedNameStore($nameStarts, $nameRecords, $resolvedName, 1);
        $triviaStore = new TriviaStore(pack('CNN', 4, 0, 10), 1);
        $literalStringStore = new LiteralStringStore(pack('N3', 1, 0, 7), 'decoded', 1);
        $sourceFile = new SourceFile(
            PHPVersion::fromParts(8, 3),
            'fixture.php',
            '0123456789',
            [1, 2],
            $nodeStore,
            $nameStore,
            $triviaStore,
            $literalStringStore,
            new ScopeStore(pack('N2', 0, 4) . pack('N2', 0, 0), 'Acme', 2),
        );

        $targets = $sourceFile->getTargetNodes();
        self::assertCount(2, $targets);
        self::assertSame(NodeKind::FunctionCall, $targets[0]->kind);
        self::assertCount(3, $sourceFile->getNodes());
        self::assertSame($targets, $sourceFile->getNodes(NodeKind::FunctionCall));
        self::assertSame($targets, $sourceFile->getChildren($sourceFile->getNode(0)));
        self::assertSame(1, $sourceFile->getFirstDescendant($sourceFile->getNode(0), NodeKind::FunctionCall)?->id);
        self::assertNull($sourceFile->getFirstDescendant($sourceFile->getNode(0), NodeKind::LiteralString));
        self::assertSame(0, $sourceFile->getParent($targets[0])?->id);
        self::assertSame('123', $sourceFile->getText($targets[0]));
        self::assertSame($resolvedName, $sourceFile->getResolvedName($targets[0])?->name);
        self::assertSame('decoded', $sourceFile->getLiteralString($targets[0]));
        self::assertNull($sourceFile->getLiteralString($targets[1]));
        self::assertSame(TriviaKind::DocBlockComment, $sourceFile->getTrivia()[0]->kind);
        self::assertSame('Acme', $sourceFile->getEnclosingClassName($targets[0]));
        self::assertNull($sourceFile->getEnclosingClassName($targets[1]));
        self::assertNull($sourceFile->getEnclosingClassName($sourceFile->getNode(0)));
    }

    /**
     * A one-target snapshot is the case that broke: the decoded target list is
     * keyed by `unpack()`, which numbers from one for several targets and from
     * zero for exactly one, while scope records are always zero-based.
     */
    public function testTheEnclosingClassOfASingleTargetIsFound(): void
    {
        $noNode = 4_294_967_295;
        $nodeStore = new NodeStore(
            [NodeKind::Program, NodeKind::MethodCall],
            pack('CNNNNN', 0, 0, 10, $noNode, 1, $noNode) . pack('CNNNNN', 1, 1, 4, 0, $noNode, $noNode),
            2,
        );
        $sourceFile = new SourceFile(
            PHPVersion::fromParts(8, 1),
            'fixture.php',
            '0123456789',
            [1],
            $nodeStore,
            new ResolvedNameStore('', '', '', 0),
            new TriviaStore('', 0),
            null,
            new ScopeStore(pack('N2', 0, 10), 'Acme\\Thing', 1),
        );

        $target = $sourceFile->getTargetNodes()[0];

        self::assertSame('Acme\\Thing', $sourceFile->getEnclosingClassName($target));
        self::assertNull($sourceFile->getEnclosingClassName($sourceFile->getNode(0)));
    }
}
