<?php

declare(strict_types=1);

namespace Mago\Sdk\Internal\Syntax;

use function array_key_exists;
use function substr;
use function unpack;

/**
 * Packed enclosing-class names, one record per target node. A resolved name is
 * never empty, so a zero-length record means no enclosing class.
 *
 * @internal
 */
final class ScopeStore
{
    public const RECORD_SIZE = 8;

    /**
     * @var array<int<0, max>, string|null>
     */
    private array $names = [];

    /**
     * @param int<0, 4294967295> $recordCount
     */
    public function __construct(
        private readonly string $records,
        private readonly string $bytes,
        private readonly int $recordCount,
    ) {}

    /**
     * @param int<0, max> $index Target index, not node identifier.
     */
    public function find(int $index): ?string
    {
        if ($index < 0 || $index >= $this->recordCount) {
            return null;
        }

        if (array_key_exists($index, $this->names)) {
            return $this->names[$index];
        }

        /** @var array{1: int<0, 4294967295>, 2: int<0, 4294967295>} $record */
        $record = unpack('N2', $this->records, $index * self::RECORD_SIZE);

        return $this->names[$index] = $record[2] === 0 ? null : substr($this->bytes, $record[1], $record[2]);
    }
}
