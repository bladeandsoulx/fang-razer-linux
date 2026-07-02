// Generates the Fang app icons (PNG + ICO) with zero dependencies:
// dark rounded square, green fang mark. Run: node scripts/gen-icons.mjs
import { deflateSync } from 'node:zlib';
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const OUT = join(dirname(fileURLToPath(import.meta.url)), '..', 'src-tauri', 'icons');
mkdirSync(OUT, { recursive: true });

// ---- PNG encoding -----------------------------------------------------------

const CRC_TABLE = Array.from({ length: 256 }, (_, n) => {
  let c = n;
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  return c >>> 0;
});

function crc32(buf) {
  let c = 0xffffffff;
  for (const b of buf) c = CRC_TABLE[(c ^ b) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const body = Buffer.concat([Buffer.from(type, 'ascii'), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(body));
  return Buffer.concat([len, body, crc]);
}

function pngEncode(size, rgba) {
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(size, 0);
  ihdr.writeUInt32BE(size, 4);
  ihdr[8] = 8; // bit depth
  ihdr[9] = 6; // RGBA
  const raw = Buffer.alloc(size * (size * 4 + 1));
  for (let y = 0; y < size; y++) {
    raw[y * (size * 4 + 1)] = 0; // filter: none
    rgba.copy(raw, y * (size * 4 + 1) + 1, y * size * 4, (y + 1) * size * 4);
  }
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk('IHDR', ihdr),
    chunk('IDAT', deflateSync(raw, { level: 9 })),
    chunk('IEND', Buffer.alloc(0))
  ]);
}

// ---- drawing ----------------------------------------------------------------

// Fang polygon in a 48x48 design space (same mark as the UI logo).
const FANG = [
  [10, 8],
  [24, 40],
  [27, 26],
  [38, 8],
  [30, 8],
  [25, 18],
  [19, 8]
];

function inPolygon(x, y, poly) {
  let inside = false;
  for (let i = 0, j = poly.length - 1; i < poly.length; j = i++) {
    const [xi, yi] = poly[i];
    const [xj, yj] = poly[j];
    if (yi > y !== yj > y && x < ((xj - xi) * (y - yi)) / (yj - yi) + xi) inside = !inside;
  }
  return inside;
}

// Rounded-rect test: distance from the point to the inner rect [r, size-r]².
function inRoundedSquare(x, y, size, r) {
  const cx = Math.max(r, Math.min(size - r, x));
  const cy = Math.max(r, Math.min(size - r, y));
  return Math.hypot(x - cx, y - cy) <= r;
}

const BG = [13, 17, 19]; // #0d1113
const GREEN = [68, 214, 44]; // #44d62c
const SS = 4; // 4x4 supersampling

function drawIcon(size) {
  const rgba = Buffer.alloc(size * size * 4);
  const radius = size * 0.22;
  for (let py = 0; py < size; py++) {
    for (let px = 0; px < size; px++) {
      let bgHits = 0;
      let fangHits = 0;
      for (let sy = 0; sy < SS; sy++) {
        for (let sx = 0; sx < SS; sx++) {
          const x = px + (sx + 0.5) / SS;
          const y = py + (sy + 0.5) / SS;
          if (!inRoundedSquare(x, y, size, radius)) continue;
          bgHits++;
          if (inPolygon((x / size) * 48, (y / size) * 48, FANG)) fangHits++;
        }
      }
      const total = SS * SS;
      const alpha = bgHits / total;
      const mix = fangHits / Math.max(1, bgHits);
      const o = (py * size + px) * 4;
      rgba[o] = Math.round(BG[0] + (GREEN[0] - BG[0]) * mix);
      rgba[o + 1] = Math.round(BG[1] + (GREEN[1] - BG[1]) * mix);
      rgba[o + 2] = Math.round(BG[2] + (GREEN[2] - BG[2]) * mix);
      rgba[o + 3] = Math.round(alpha * 255);
    }
  }
  return pngEncode(size, rgba);
}

// ---- outputs ----------------------------------------------------------------

const png256 = drawIcon(256);
writeFileSync(join(OUT, '32x32.png'), drawIcon(32));
writeFileSync(join(OUT, '128x128.png'), drawIcon(128));
writeFileSync(join(OUT, '128x128@2x.png'), png256);
writeFileSync(join(OUT, 'icon.png'), drawIcon(512));

// ICO with a single PNG-compressed 256px entry (valid since Vista).
const entry = Buffer.alloc(16);
entry[0] = 0; // width 256
entry[1] = 0; // height 256
entry.writeUInt16LE(1, 4); // planes
entry.writeUInt16LE(32, 6); // bpp
entry.writeUInt32LE(png256.length, 8);
entry.writeUInt32LE(22, 12); // offset
const header = Buffer.alloc(6);
header.writeUInt16LE(1, 2); // type: icon
header.writeUInt16LE(1, 4); // count
writeFileSync(join(OUT, 'icon.ico'), Buffer.concat([header, entry, png256]));

console.log('icons written to', OUT);
