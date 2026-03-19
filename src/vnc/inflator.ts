// @ts-nocheck
/*
 * noVNC: HTML5 VNC client
 * Copyright (C) 2020 The noVNC Authors
 * Licensed under MPL 2.0 (see LICENSE.txt)
 *
 * Rewritten to use fflate instead of bundled zlib.
 *
 * VNC Tight encoding uses persistent zlib streams (Z_NO_FLUSH / Z_SYNC_FLUSH)
 * with standard zlib wrapping (2-byte header).  fflate's `Decompress` auto-
 * detects the zlib header on the first push and then maintains streaming
 * inflate state across subsequent pushes – exactly what we need.
 */

import { Decompress } from 'fflate';

export default class Inflator {
    private _chunks: Uint8Array[] = [];
    private _totalLen = 0;
    private _inflator: Decompress;

    constructor() {
        this._inflator = this._createStream();
    }

    private _createStream(): Decompress {
        const s = new Decompress();
        s.ondata = (chunk: Uint8Array, _final: boolean) => {
            this._chunks.push(chunk);
            this._totalLen += chunk.length;
        };
        return s;
    }

    setInput(data: Uint8Array | null): void {
        if (!data) {
            // Just clear the reference; don't touch the stream state
            return;
        }

        // Push compressed data through the persistent streaming decompressor.
        // `false` = not final, stream continues across future setInput calls.
        this._inflator.push(data, false);
    }

    inflate(expected: number): Uint8Array {
        if (this._totalLen < expected) {
            throw new Error("Incomplete zlib block");
        }

        // Fast path: single chunk with exact or larger size
        if (this._chunks.length === 1 && this._chunks[0].length >= expected) {
            const result = this._chunks[0].subarray(0, expected);
            if (this._chunks[0].length === expected) {
                this._chunks.shift();
            } else {
                this._chunks[0] = this._chunks[0].subarray(expected);
            }
            this._totalLen -= expected;
            return new Uint8Array(result);
        }

        // Slow path: merge chunks
        const out = new Uint8Array(expected);
        let offset = 0;
        while (offset < expected) {
            const chunk = this._chunks[0];
            const needed = expected - offset;
            if (chunk.length <= needed) {
                out.set(chunk, offset);
                offset += chunk.length;
                this._chunks.shift();
            } else {
                out.set(chunk.subarray(0, needed), offset);
                this._chunks[0] = chunk.subarray(needed);
                offset += needed;
            }
        }
        this._totalLen -= expected;
        return out;
    }

    reset(): void {
        this._chunks = [];
        this._totalLen = 0;
        this._inflator = this._createStream();
    }
}
