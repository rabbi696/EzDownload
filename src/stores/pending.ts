import { defineStore } from "pinia";
import { useSettingStore } from "@/stores/setting";
import { getCodecKey } from "@/utils/formats";
import type { FetchedVideoData, PendingItem, VideoFormat } from "@/types";

const generateId = () => `pd_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;

const selectVideoFormat = (
  formats: VideoFormat[],
  maxHeight?: number,
  preferH264 = false,
): string => {
  if (preferH264) {
    const h264List = formats.filter(
      (f) => getCodecKey(f.vcodec) === "h264" && !f.ext.toLowerCase().includes("webm"),
    );
    const targetList = h264List.length > 0 ? h264List : formats;
    if (!maxHeight) return targetList[0]?.format_id ?? "";
    return (
      targetList.find((format) => format.height != null && format.height <= maxHeight)
        ?.format_id ??
      targetList[0]?.format_id ??
      ""
    );
  }
  if (!maxHeight) return formats[0]?.format_id ?? "";
  return (
    formats.find((format) => format.height != null && format.height <= maxHeight)?.format_id ??
    formats[0]?.format_id ??
    ""
  );
};

const selectAudioFormat = (formats: VideoFormat[], preferAac = false): string => {
  if (preferAac) {
    const aac = formats.find((f) => getCodecKey(f.acodec) === "aac");
    if (aac) return aac.format_id;
  }
  return formats[0]?.format_id ?? "";
};

export const createPendingItem = (data: FetchedVideoData, quick = false): PendingItem => {
  const settingStore = useSettingStore();
  const maxHeight = quick ? settingStore.quickMaxHeight : undefined;
  const premierePreset = settingStore.premierePresetDefault;

  const hasH264 = data.videoFormats.some(
    (f) => getCodecKey(f.vcodec) === "h264" && !f.ext.toLowerCase().includes("webm"),
  );
  let autoConvertTarget = settingStore.autoConvertIncompatible;
  if (premierePreset && !hasH264 && autoConvertTarget === "off") {
    autoConvertTarget = "h264_mp4";
  }

  return {
    ...data,
    id: generateId(),
    createdAt: Date.now(),
    selectedPlaylistItems: data.isPlaylist ? data.playlistEntries.map((_, i) => i + 1) : [],
    downloadMode: quick ? settingStore.quickDownloadMode : "default",
    selectedVideoFormat: selectVideoFormat(data.videoFormats, maxHeight, premierePreset),
    selectedAudioFormat: selectAudioFormat(data.audioFormats, premierePreset),
    startTime: null,
    endTime: null,
    embedSubs: false,
    embedThumbnail: quick ? settingStore.quickEmbedThumbnail : false,
    embedMetadata: quick ? settingStore.quickEmbedMetadata : false,
    embedChapters: quick ? settingStore.quickEmbedChapters : false,
    sponsorblockRemove: quick ? settingStore.quickSponsorblockRemove : false,
    extractAudio: false,
    audioConvertFormat: "",
    noMerge: quick ? settingStore.quickNoMerge : false,
    recodeFormat: quick ? settingStore.quickRecodeFormat : "",
    limitRate: quick ? settingStore.quickLimitRate : "",
    ffmpegArgs: quick ? settingStore.quickFfmpegArgs : settingStore.defaultFfmpegArgs,
    selectedSubtitles: [],
    liveFromStart:
      data.videoInfo.is_live === true || data.videoInfo.live_status === "is_live",
    premierePreset,
    autoConvertTarget,
  };
};

export const usePendingStore = defineStore("pending", () => {
  const items = ref<PendingItem[]>([]);
  const activeId = ref<string>("");

  const activeItem = computed<PendingItem | null>(
    () => items.value.find((i) => i.id === activeId.value) ?? null,
  );

  const add = (data: FetchedVideoData): string => {
    const item = createPendingItem(data);
    items.value.push(item);
    activeId.value = item.id;
    return item.id;
  };

  const remove = (id: string) => {
    const idx = items.value.findIndex((i) => i.id === id);
    if (idx === -1) return;
    items.value.splice(idx, 1);
    if (activeId.value === id) {
      const next = items.value[idx] ?? items.value[idx - 1] ?? items.value[0];
      activeId.value = next ? next.id : "";
    }
  };

  /** 刷新当前项：替换源数据并重置依赖源数据的派生字段（格式/分P 选中），保留用户填的额外选项 */
  const refresh = (id: string, data: FetchedVideoData) => {
    const item = items.value.find((i) => i.id === id);
    if (!item) return;
    item.url = data.url;
    item.videoInfo = data.videoInfo;
    item.videoFormats = data.videoFormats;
    item.audioFormats = data.audioFormats;
    item.isPlaylist = data.isPlaylist;
    item.playlistEntries = data.playlistEntries;
    item.selectedPlaylistItems = data.isPlaylist ? data.playlistEntries.map((_, i) => i + 1) : [];
    item.selectedVideoFormat = data.videoFormats[0]?.format_id ?? "";
    item.selectedAudioFormat = data.audioFormats[0]?.format_id ?? "";
  };

  const clear = () => {
    items.value = [];
    activeId.value = "";
  };

  return {
    items,
    activeId,
    activeItem,
    add,
    remove,
    refresh,
    clear,
  };
});
