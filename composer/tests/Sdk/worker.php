<?php

declare(strict_types=1);

use Mago\Sdk\Extension;
use Mago\Sdk\Worker;
use Mago\Tests\Sdk\Fixture\ArgumentViewRule;
use Mago\Tests\Sdk\Fixture\EnclosingClassRule;
use Mago\Tests\Sdk\Fixture\NoInterfaceRule;
use Mago\Tests\Sdk\Fixture\PreferArrayAnyRule;

require_once __DIR__ . '/../../../vendor/autoload.php';
require_once __DIR__ . '/Fixture/ArgumentViewRule.php';
require_once __DIR__ . '/Fixture/EnclosingClassRule.php';
require_once __DIR__ . '/Fixture/PreferArrayAnyRule.php';
require_once __DIR__ . '/Fixture/NoInterfaceRule.php';

$worker = new Worker(
    new Extension(
        identifier: 'mago/sdk-iter-test',
        name: 'Mago SDK Iter Test',
        version: '0.0.0',
        linterRules: [new PreferArrayAnyRule()],
    ),
    new Extension(
        identifier: 'mago/sdk-interface-test',
        name: 'Mago SDK Interface Test',
        version: '0.0.0',
        linterRules: [new NoInterfaceRule()],
    ),
    new Extension(
        identifier: 'mago/sdk-argument-test',
        name: 'Mago SDK Argument Test',
        version: '0.0.0',
        linterRules: [new ArgumentViewRule()],
    ),
    new Extension(
        identifier: 'mago/sdk-scope-test',
        name: 'Mago SDK Scope Test',
        version: '0.0.0',
        linterRules: [new EnclosingClassRule()],
    ),
);
$worker->run();
