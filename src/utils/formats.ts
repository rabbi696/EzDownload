import type { VideoFormat } from "@/types";

export const getCodecKey = (codec: string): string => {
  const normalized = codec.toLowerCase();
  if (/^(avc1|avc3|h264)/.test(normalized)) return "h264";
  if (/^(hev1|hvc1|h265|hevc)/.test(normalized)) return "hevc";
  if (/^(av01|av1)/.test(normalized)) return "av1";
  if (/^(vp09|vp9)/.test(normalized)) return "vp9";
  if (/^vp8/.test(normalized)) return "vp8";
  if (/^(prores|apch|apcn|apcs|apco|ap4h|ap4x)/.test(normalized)) return "prores";
  if (/^(mp4a|aac)/.test(normalized)) return "aac";
  if (/^(opus)/.test(normalized)) return "opus";
  if (/^(vorbis)/.test(normalized)) return "vorbis";
  if (/^(mp3)/.test(normalized)) return "mp3";
  if (/^(ec-3|eac3)/.test(normalized)) return "eac3";
  if (/^(ac-3|ac3)/.test(normalized)) return "ac3";
  return normalized.split(".")[0] || "unknown";
};

const CODEC_LABELS: Record<string, string> = {
  h264: "H.264",
  prores: "Apple ProRes",
  hevc: "H.265 / HEVC",
  av1: "AV1",
  vp9: "VP9",
  vp8: "VP8",
  aac: "AAC",
  opus: "Opus",
  vorbis: "Vorbis",
  mp3: "MP3",
  eac3: "E-AC-3",
  ac3: "AC-3",
  unknown: "Unknown",
};

export const getCodecLabel = (codec: string): string => {
  const key = getCodecKey(codec);
  return CODEC_LABELS[key] || key.toUpperCase();
};

export const PREMIERE_READY_SELECTOR =
  "bv*[vcodec^=avc1]+ba[acodec^=mp4a]/b[vcodec^=avc1]";

export type CodecCompatibility = "ready" | "convert_recommended" | "unknown";

export const isPremiereReadyCodec = (
  vcodec?: string | null,
  container?: string | null,
): boolean => {
  if (!vcodec) return false;
  const key = getCodecKey(vcodec);
  const isWebm = (container || "").toLowerCase().includes("webm");
  if (isWebm) return false;
  return key === "h264" || key === "prores";
};

export const getCodecCompatibility = (
  vcodec?: string | null,
  container?: string | null,
): CodecCompatibility => {
  if (isPremiereReadyCodec(vcodec, container)) {
    return "ready";
  }
  const isWebm = (container || "").toLowerCase().includes("webm");
  const key = vcodec ? getCodecKey(vcodec) : "";
  if (isWebm || key === "av1" || key === "vp9" || key === "vp8") {
    return "convert_recommended";
  }
  return "unknown";
};

/**
 * 视频格式排序：
 * 1. 分辨率 (height) 降序
 * 2. 宽度 (width) 降序
 * 3. 帧率 (fps) 降序
 * 4. Premiere Ready 格式优先 (H.264/ProRes 且非 webm)
 * 5. 已知大小优先 (filesize/filesize_approx > 0 排在未知大小前面，避免选到低码率 Unknown size 格式)
 * 6. 大小/码率降序 (已知大文件/高码率优先)
 */
export const compareVideoFormats = (a: VideoFormat, b: VideoFormat): number => {
  // 1. 分辨率高度降序
  const heightA = a.height || 0;
  const heightB = b.height || 0;
  if (heightB !== heightA) return heightB - heightA;

  // 2. 分辨率宽度降序
  const widthA = a.width || 0;
  const widthB = b.width || 0;
  if (widthB !== widthA) return widthB - widthA;

  // 3. 帧率降序
  const fpsA = a.fps || 0;
  const fpsB = b.fps || 0;
  if (fpsB !== fpsA) return fpsB - fpsA;

  // 4. Premiere Ready 兼容优先 (avc1/prores mp4 优先于 vp9/av1 webm)
  const isReadyA = isPremiereReadyCodec(a.vcodec, a.ext) ? 1 : 0;
  const isReadyB = isPremiereReadyCodec(b.vcodec, b.ext) ? 1 : 0;
  if (isReadyB !== isReadyA) return isReadyB - isReadyA;

  // 5. 已知大小优先于未知大小 (例如优先选择明确有 45.7MB 的 #137，而非 Unknown size 的 #270)
  const sizeA = a.filesize || a.filesize_approx || 0;
  const sizeB = b.filesize || b.filesize_approx || 0;
  const hasSizeA = sizeA > 0 ? 1 : 0;
  const hasSizeB = sizeB > 0 ? 1 : 0;
  if (hasSizeB !== hasSizeA) return hasSizeB - hasSizeA;

  // 6. 文件大小降序（同等分辨率下，体积大往往代表更高质量/码率流）
  if (sizeB !== sizeA) return sizeB - sizeA;

  // 7. 总码率降序
  const tbrA = a.tbr || 0;
  const tbrB = b.tbr || 0;
  if (tbrB !== tbrA) return tbrB - tbrA;

  return 0;
};

export const findHighestH264Format = (formats: VideoFormat[]): VideoFormat | undefined => {
  const h264List = formats.filter(
    (f) => getCodecKey(f.vcodec) === "h264" && !f.ext.toLowerCase().includes("webm"),
  );
  return [...h264List].sort(compareVideoFormats)[0];
};

export const findHighestFormat = (formats: VideoFormat[]): VideoFormat | undefined => {
  return [...formats].sort(compareVideoFormats)[0];
};

export const checkH264ResolutionCapped = (formats: VideoFormat[]) => {
  const highestH264 = findHighestH264Format(formats);
  const highestOverall = findHighestFormat(formats);
  const h264MaxHeight = highestH264?.height || 0;
  const overallMaxHeight = highestOverall?.height || 0;
  const isCapped = overallMaxHeight > h264MaxHeight && h264MaxHeight > 0;
  return {
    isCapped,
    h264MaxHeight,
    overallMaxHeight,
    overallCodec: highestOverall ? getCodecLabel(highestOverall.vcodec) : "",
    highestH264Id: highestH264?.format_id,
    highestOverallId: highestOverall?.format_id,
  };
};

const audioRoleRank = (format: VideoFormat): number => {
  const description = `${format.format_note || ""} ${format.format || ""}`.toLowerCase();
  if (description.includes("original")) return 3;
  if (description.includes("audio description") || description.includes("descriptive")) return -2;
  if (description.includes("dubbed") || description.includes("translated")) return -1;
  if (description.includes("default")) return 2;
  return 0;
};

/**
 * 保留 yt-dlp 的语言偏好语义，优先原声，再按是否 DRC、码率排序。
 * 旧逻辑只比较码率，会把高码率配音轨误设为默认值。
 */
export const compareAudioFormats = (a: VideoFormat, b: VideoFormat): number => {
  if (a.language_preference != null && b.language_preference != null) {
    const preferenceDifference = b.language_preference - a.language_preference;
    if (preferenceDifference !== 0) return preferenceDifference;
  }

  const roleDifference = audioRoleRank(b) - audioRoleRank(a);
  if (roleDifference !== 0) return roleDifference;

  const aDrc = /\bdrc\b/i.test(a.format_note || "") ? 1 : 0;
  const bDrc = /\bdrc\b/i.test(b.format_note || "") ? 1 : 0;
  if (aDrc !== bDrc) return aDrc - bDrc;

  return (b.abr || b.tbr || 0) - (a.abr || a.tbr || 0);
};
