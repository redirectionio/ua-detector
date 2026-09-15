<?php

namespace device_detector;

use Castor\Attribute\AsTask;

use function Castor\context;
use function Castor\run;

/**
 * Commit of matomo-org/device-detector the test fixtures are taken from.
 *
 * Only the fixtures are: they say what a user agent means, which is the specification this
 * library answers. How matomo goes about answering it is its own business, and its regexes are
 * under a licence we do not want to carry.
 */
const MATOMO_REVISION = '5a6f5d0184b5a867c1db9c0733f6b89686c5c47b';

#[AsTask(description: 'Syncs the matomo device-detector test fixtures', name: 'sync', namespace: 'matomo')]
function sync()
{
    $gitRepository = 'https://github.com/matomo-org/device-detector.git';
    $tempDir = sys_get_temp_dir() . '/device-detector-' . uniqid();

    run("git clone --quiet $gitRepository $tempDir");
    run('git checkout --quiet ' . MATOMO_REVISION, context()->withWorkingDirectory($tempDir));

    $fixturesDir = __DIR__ . '/tests/matomo-device-detector/fixtures';
    run("rm -rf $fixturesDir");
    run("mkdir -p $fixturesDir");
    run("cp -r $tempDir/Tests/fixtures/* $fixturesDir");

    // Matomo also tests each of its parsers on its own, and those fixtures name one thing each:
    // a client, or an operating system, and nothing else. That is the shape of `src/devices`'s
    // axis files, so they are worth having even though they say less than the ones above.
    $axisDir = __DIR__ . '/tests/matomo-device-detector/axis';
    run("rm -rf $axisDir");
    run("mkdir -p $axisDir");

    foreach (['Parser', 'Parser/Client', 'Parser/Device'] as $parser) {
        $from = "$tempDir/Tests/$parser/fixtures";

        if (is_dir($from)) {
            $into = $axisDir . '/' . str_replace('/', '-', strtolower($parser));
            run("mkdir -p $into");
            run("cp -r $from/* $into");
        }
    }

    run("rm -rf $tempDir");
}
