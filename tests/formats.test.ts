import test from "node:test";
import assert from "node:assert/strict";
import {
  getCodecKey,
  getCodecLabel,
  isPremiereReadyCodec,
  getCodecCompatibility,
  findHighestH264Format,
  findHighestFormat,
  compareVideoFormats,
  checkH264ResolutionCapped,
  PREMIERE_READY_SELECTOR,
} from "../src/utils/formats.ts";
import type { VideoFormat } from "../src/types/index.ts";

test("getCodecKey normalizes common video and audio codecs", () => {
  assert.equal(getCodecKey("avc1.640028"), "h264");
  assert.equal(getCodecKey("h264"), "h264");
  assert.equal(getCodecKey("av01.0.08M.08"), "av1");
  assert.equal(getCodecKey("vp09.00.51.08"), "vp9");
  assert.equal(getCodecKey("hev1.1.6.L93.B0"), "hevc");
  assert.equal(getCodecKey("prores"), "prores");
  assert.equal(getCodecKey("apcn"), "prores");
  assert.equal(getCodecKey("mp4a.40.2"), "aac");
  assert.equal(getCodecKey("opus"), "opus");
});

test("isPremiereReadyCodec identifies Premiere-compatible codecs", () => {
  assert.equal(isPremiereReadyCodec("avc1.640028", "mp4"), true);
  assert.equal(isPremiereReadyCodec("h264", "mp4"), true);
  assert.equal(isPremiereReadyCodec("prores", "mov"), true);

  // AV1 and VP9 are not natively Premiere ready
  assert.equal(isPremiereReadyCodec("av01.0.08M.08", "mp4"), false);
  assert.equal(isPremiereReadyCodec("vp09.00.51.08", "webm"), false);

  // Any codec inside webm container is rejected
  assert.equal(isPremiereReadyCodec("avc1.640028", "webm"), false);
});

test("getCodecCompatibility returns correct status", () => {
  assert.equal(getCodecCompatibility("avc1.640028", "mp4"), "ready");
  assert.equal(getCodecCompatibility("prores", "mov"), "ready");
  assert.equal(getCodecCompatibility("av01.0.08M.08", "mp4"), "convert_recommended");
  assert.equal(getCodecCompatibility("vp09.00.51.08", "webm"), "convert_recommended");
});

test("findHighestH264Format selects highest resolution non-webm H.264 stream", () => {
  const formats: VideoFormat[] = [
    {
      format_id: "137",
      vcodec: "avc1.640028",
      acodec: "none",
      ext: "mp4",
      height: 1080,
      width: 1920,
      fps: 30,
      resolution: "1920x1080",
      filesize: 50000000,
      filesize_approx: null,
      format_note: "1080p",
      tbr: 4000,
      abr: null,
    },
    {
      format_id: "136",
      vcodec: "avc1.4d401f",
      acodec: "none",
      ext: "mp4",
      height: 720,
      width: 1280,
      fps: 30,
      resolution: "1280x720",
      filesize: 25000000,
      filesize_approx: null,
      format_note: "720p",
      tbr: 2000,
      abr: null,
    },
    {
      format_id: "401",
      vcodec: "av01.0.12M.08",
      acodec: "none",
      ext: "mp4",
      height: 2160,
      width: 3840,
      fps: 60,
      resolution: "3840x2160",
      filesize: 120000000,
      filesize_approx: null,
      format_note: "2160p60",
      tbr: 12000,
      abr: null,
    },
  ];

  const highestH264 = findHighestH264Format(formats);
  assert.equal(highestH264?.format_id, "137");
  assert.equal(highestH264?.height, 1080);
});

test("checkH264ResolutionCapped accurately flags 4K AV1 capping", () => {
  const formats: VideoFormat[] = [
    {
      format_id: "137",
      vcodec: "avc1.640028",
      acodec: "none",
      ext: "mp4",
      height: 1080,
      width: 1920,
      fps: 30,
      resolution: "1920x1080",
      filesize: 50000000,
      filesize_approx: null,
      format_note: "1080p",
      tbr: 4000,
      abr: null,
    },
    {
      format_id: "401",
      vcodec: "av01.0.12M.08",
      acodec: "none",
      ext: "mp4",
      height: 2160,
      width: 3840,
      fps: 60,
      resolution: "3840x2160",
      filesize: 120000000,
      filesize_approx: null,
      format_note: "2160p60",
      tbr: 12000,
      abr: null,
    },
  ];

  const info = checkH264ResolutionCapped(formats);
  assert.equal(info.isCapped, true);
  assert.equal(info.h264MaxHeight, 1080);
  assert.equal(info.overallMaxHeight, 2160);
  assert.equal(info.overallCodec, "AV1");
});

test("PREMIERE_READY_SELECTOR has correct yt-dlp format strategy", () => {
  assert.ok(PREMIERE_READY_SELECTOR.includes("vcodec^=avc1"));
  assert.ok(PREMIERE_READY_SELECTOR.includes("acodec^=mp4a"));
});

test("compareVideoFormats prefers known filesize and higher quality over unknown size", () => {
  // Scenario matching user's case: Format #270 (Unknown size) vs Format #137 (45.7 MB)
  const format270: VideoFormat = {
    format_id: "270",
    vcodec: "avc1.640028",
    acodec: "none",
    ext: "mp4",
    height: 1080,
    width: 1920,
    fps: 25,
    resolution: "1920x1080",
    filesize: null,
    filesize_approx: null,
    format_note: "1080p25",
    tbr: null,
    abr: null,
  };

  const format137: VideoFormat = {
    format_id: "137",
    vcodec: "avc1.640028",
    acodec: "none",
    ext: "mp4",
    height: 1080,
    width: 1920,
    fps: 25,
    resolution: "1920x1080",
    filesize: 47919920, // 45.7 MB
    filesize_approx: null,
    format_note: "1080p25",
    tbr: 2500,
    abr: null,
  };

  const format271: VideoFormat = {
    format_id: "271",
    vcodec: "vp09.00.51.08",
    acodec: "none",
    ext: "webm",
    height: 1440,
    width: 2560,
    fps: 25,
    resolution: "2560x1440",
    filesize: 89653248, // 85.5 MB
    filesize_approx: null,
    format_note: "1440p25",
    tbr: 4500,
    abr: null,
  };

  const format248: VideoFormat = {
    format_id: "248",
    vcodec: "vp09.00.51.08",
    acodec: "none",
    ext: "webm",
    height: 1080,
    width: 1920,
    fps: 25,
    resolution: "1920x1080",
    filesize: 32400998, // 30.9 MB
    filesize_approx: null,
    format_note: "1080p25",
    tbr: 1800,
    abr: null,
  };

  // 1. When comparing 137 and 270: 137 (known size 45.7MB) must rank higher than 270 (unknown size)
  const diff = compareVideoFormats(format270, format137);
  assert.ok(diff > 0, "Format with known filesize must rank before unknown size");

  // 2. Sorting array containing both: 137 must be first among 1080p H.264
  const list = [format270, format137].sort(compareVideoFormats);
  assert.equal(list[0].format_id, "137");
  assert.equal(list[1].format_id, "270");

  // 3. findHighestH264Format must choose #137 over #270
  const highestH264 = findHighestH264Format([format271, format270, format137, format248]);
  assert.equal(highestH264?.format_id, "137");

  // 4. At 1080p, Premiere-ready H.264 (#137) ranks before VP9 (#248)
  const sorted1080 = [format248, format137].sort(compareVideoFormats);
  assert.equal(sorted1080[0].format_id, "137");
});
