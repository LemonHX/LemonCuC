// @ts-nocheck
/*
 * noVNC: HTML5 VNC client
 * Copyright (C) 2020 The noVNC Authors
 * Licensed under MPL 2.0 (see LICENSE.txt)
 *
 * Rewritten to use fflate instead of bundled zlib.
 */

import { zlibSync } from 'fflate';

export default class Deflator {
    deflate(inData: Uint8Array): Uint8Array {
        // zlibSync performs a one-shot zlib deflate (with header).
        // The old code used Z_FULL_FLUSH which resets the compressor state
        // after each call, so one-shot is semantically equivalent.
        return zlibSync(inData, { level: 5 });
    }
}
