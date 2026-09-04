<script setup lang="ts">
import { formatFileSize } from "@/utils/format";
import {
  getCodecKey,
  getCodecLabel,
  isPremiereReadyCodec,
  getCodecCompatibility,
  checkH264ResolutionCapped,
  findHighestH264Format,
} from "@/utils/formats";
import { useI18n } from "vue-i18n";
import type { VideoFormat, VideoInfo } from "@/types";

const { t } = useI18n();

const props = defineProps<{
  videoFormats: VideoFormat[];
  audioFormats: VideoFormat[];
  videoInfo: VideoInfo;
}>();

const downloadMode = defineModel<"default" | "video" | "audio">("downloadMode", {
  required: true,
});
const selectedVideoFormat = defineModel<string>("selectedVideoFormat", {
  required: true,
});
const selectedAudioFormat = defineModel<string>("selectedAudioFormat", {
  required: true,
});
const premierePreset = defineModel<boolean>("premierePreset", {
  default: false,
});
const autoConvertTarget = defineModel<"off" | "h264_mp4" | "prores_422_lt_mov">("autoConvertTarget", {
  default: "off",
});

const selectedVideoCodec = ref("all");
const selectedAudioCodec = ref("all");

const currentSelectedVideoFormat = computed(() =>
  props.videoFormats.find((f) => f.format_id === selectedVideoFormat.value),
);

const selectedFormatCompatibility = computed(() => {
  if (downloadMode.value === "audio") return "ready";
  if (!currentSelectedVideoFormat.value) return "unknown";
  return getCodecCompatibility(
    currentSelectedVideoFormat.value.vcodec,
    currentSelectedVideoFormat.value.ext,
  );
});

const h264CapInfo = computed(() => checkH264ResolutionCapped(props.videoFormats));

const handleSwitchToH264 = () => {
  const highestH264 = findHighestH264Format(props.videoFormats);
  if (highestH264) {
    selectedVideoFormat.value = highestH264.format_id;
    selectedVideoCodec.value = "all";
    autoConvertTarget.value = "off";
  }
};

const handleSetAutoConvert = (target: "h264_mp4" | "prores_422_lt_mov") => {
  if (autoConvertTarget.value === target) {
    autoConvertTarget.value = "off";
  } else {
    autoConvertTarget.value = target;
  }
};

const createCodecOptions = (formats: VideoFormat[]) => {
  const codecs = new Map<string, string>();
  for (const format of formats) {
    const codec = format.vcodec !== "none" ? format.vcodec : format.acodec;
    codecs.set(getCodecKey(codec), getCodecLabel(codec));
  }
  return [
    { label: t("detail.allCodecs"), value: "all" },
    ...Array.from(codecs, ([value, label]) => ({ value, label })),
  ];
};

const videoCodecOptions = computed(() => createCodecOptions(props.videoFormats));
const audioCodecOptions = computed(() => createCodecOptions(props.audioFormats));

const filteredVideoFormats = computed(() =>
  selectedVideoCodec.value === "all"
    ? props.videoFormats
    : props.videoFormats.filter(
        (format) => getCodecKey(format.vcodec) === selectedVideoCodec.value,
      ),
);

const filteredAudioFormats = computed(() =>
  selectedAudioCodec.value === "all"
    ? props.audioFormats
    : props.audioFormats.filter(
        (format) => getCodecKey(format.acodec) === selectedAudioCodec.value,
      ),
);

const formatsIncomplete = computed(() => {
  if (props.audioFormats.length > 0 || props.videoFormats.length === 0) return false;
  return Math.max(...props.videoFormats.map((format) => format.height || 0)) <= 360;
});

/** 是否为正在直播 */
const isLive = computed(
  () => props.videoInfo.is_live === true || props.videoInfo.live_status === "is_live",
);

/** 视频格式下拉选项 */
const videoFormatOptions = computed(() =>
  filteredVideoFormats.value.map((f) => {
    const isReady = isPremiereReadyCodec(f.vcodec, f.ext);
    const badge = isReady ? "✓ Premiere" : "";
    return {
      label: [
        badge,
        `${f.height}p${f.fps ? ` ${f.fps}fps` : ""}`,
        getCodecLabel(f.vcodec),
        f.dynamic_range,
        f.ext,
        f.filesize || f.filesize_approx
          ? formatFileSize(f.filesize || f.filesize_approx || 0)
          : t("detail.unknownSize"),
        `#${f.format_id}`,
      ]
        .filter(Boolean)
        .join(" · "),
      value: f.format_id,
    };
  }),
);

/** 音频格式下拉选项 */
const audioFormatOptions = computed(() =>
  filteredAudioFormats.value.map((f) => {
    const isAac = getCodecKey(f.acodec) === "aac";
    const badge = isAac ? "✓ AAC" : "";
    return {
      label: [
        badge,
        f.language ? `[${f.language}]` : "",
        f.format_note,
        f.abr ? `${f.abr}kbps` : "",
        getCodecLabel(f.acodec),
        f.audio_channels ? `${f.audio_channels}ch` : "",
        f.ext,
        f.filesize || f.filesize_approx
          ? formatFileSize(f.filesize || f.filesize_approx || 0)
          : t("detail.unknownSize"),
        `#${f.format_id}`,
      ]
        .filter(Boolean)
        .filter((part, index, parts) => parts.indexOf(part) === index)
        .join(" · "),
      value: f.format_id,
    };
  }),
);

const handleVideoCodecChange = (value: string) => {
  selectedVideoCodec.value = value;
  const currentIsVisible = filteredVideoFormats.value.some(
    (format) => format.format_id === selectedVideoFormat.value,
  );
  if (!currentIsVisible) selectedVideoFormat.value = filteredVideoFormats.value[0]?.format_id || "";
};

const handleAudioCodecChange = (value: string) => {
  selectedAudioCodec.value = value;
  const currentIsVisible = filteredAudioFormats.value.some(
    (format) => format.format_id === selectedAudioFormat.value,
  );
  if (!currentIsVisible) selectedAudioFormat.value = filteredAudioFormats.value[0]?.format_id || "";
};

watch(
  () => props.videoFormats,
  () => {
    if (!videoCodecOptions.value.some((option) => option.value === selectedVideoCodec.value)) {
      selectedVideoCodec.value = "all";
    }
  },
);

watch(
  () => props.audioFormats,
  () => {
    if (!audioCodecOptions.value.some((option) => option.value === selectedAudioCodec.value)) {
      selectedAudioCodec.value = "all";
    }
  },
);

watch(
  [() => premierePreset.value, () => selectedFormatCompatibility.value],
  ([preset, compat]) => {
    if (preset && compat === "convert_recommended" && autoConvertTarget.value === "off") {
      autoConvertTarget.value = "h264_mp4";
    }
  },
  { immediate: true },
);
</script>

<template>
  <n-card :title="$t('detail.downloadMethod')" size="small">
    <n-flex vertical :size="12">
      <n-flex justify="space-between" align="center" style="padding: 2px 0">
        <n-flex align="center" :size="8">
          <n-tag :type="premierePreset ? 'success' : 'default'" size="small" round>
            {{ $t("premiere.presetBadge") }}
          </n-tag>
          <n-text strong style="font-size: 13px">
            {{ $t("premiere.presetTitle") }}
          </n-text>
        </n-flex>
        <n-switch v-model:value="premierePreset" size="small" />
      </n-flex>

      <n-text v-if="premierePreset" depth="3" style="font-size: 12px; line-height: 1.4">
        {{ $t("premiere.presetDesc") }}
      </n-text>

      <!-- Premiere Ready active: incompatible source stream warning and conversion selector -->
      <n-alert
        v-if="premierePreset && downloadMode !== 'audio' && selectedFormatCompatibility === 'convert_recommended'"
        type="warning"
        :bordered="false"
      >
        <template #header>
          <span style="font-size: 13px; font-weight: 600">
            {{ $t("premiere.fallbackChoiceTitle") }}
          </span>
        </template>
        <n-flex vertical :size="6" style="font-size: 12px; margin-top: 2px">
          <div>
            {{
              $t("premiere.fallbackChoiceDesc", {
                codec: getCodecLabel(currentSelectedVideoFormat?.vcodec || ""),
                ext: currentSelectedVideoFormat?.ext || "",
              })
            }}
          </div>
          <div v-if="!h264CapInfo.highestH264Id" style="color: var(--n-warning-color, #f0a020); font-weight: 500">
            {{ $t("premiere.noH264StreamAlert") }}
          </div>
          <div v-else-if="h264CapInfo.isCapped" style="color: var(--n-text-color-depth-2)">
            {{
              $t("premiere.h264CappedNotice", {
                h264Res: `${h264CapInfo.h264MaxHeight}p`,
                maxRes: `${h264CapInfo.overallMaxHeight}p`,
                codec: h264CapInfo.overallCodec,
              })
            }}
          </div>
          <n-flex :size="8" align="center" style="margin-top: 4px" wrap>
            <n-button
              v-if="h264CapInfo.highestH264Id"
              size="tiny"
              secondary
              type="primary"
              @click="handleSwitchToH264"
            >
              {{ $t("premiere.switchToH264", { res: `${h264CapInfo.h264MaxHeight}p` }) }}
            </n-button>
            <n-button
              size="tiny"
              :type="autoConvertTarget === 'h264_mp4' ? 'success' : 'default'"
              secondary
              @click="handleSetAutoConvert('h264_mp4')"
            >
              {{ $t("premiere.autoConvertH264") }}
              {{ autoConvertTarget === "h264_mp4" ? " ✓" : "" }}
            </n-button>
            <n-button
              size="tiny"
              :type="autoConvertTarget === 'prores_422_lt_mov' ? 'success' : 'default'"
              secondary
              @click="handleSetAutoConvert('prores_422_lt_mov')"
            >
              {{ $t("premiere.autoConvertProres") }}
              {{ autoConvertTarget === "prores_422_lt_mov" ? " ✓" : "" }}
            </n-button>
          </n-flex>
        </n-flex>
      </n-alert>

      <n-alert
        v-if="!premierePreset && downloadMode !== 'audio' && selectedFormatCompatibility === 'convert_recommended'"
        type="warning"
        :bordered="false"
      >
        <template #header>
          <span style="font-size: 13px; font-weight: 600">
            {{ $t("premiere.warningTitle") }}
          </span>
        </template>
        <n-flex vertical :size="6" style="font-size: 12px; margin-top: 2px">
          <div>
            {{
              $t("premiere.warningDesc", {
                codec: getCodecLabel(currentSelectedVideoFormat?.vcodec || ""),
                ext: currentSelectedVideoFormat?.ext || "",
              })
            }}
          </div>
          <div v-if="h264CapInfo.isCapped" style="color: var(--n-text-color-depth-2)">
            {{
              $t("premiere.h264CappedNotice", {
                h264Res: `${h264CapInfo.h264MaxHeight}p`,
                maxRes: `${h264CapInfo.overallMaxHeight}p`,
                codec: h264CapInfo.overallCodec,
              })
            }}
          </div>
          <n-flex :size="8" align="center" style="margin-top: 4px" wrap>
            <n-button
              v-if="h264CapInfo.highestH264Id"
              size="tiny"
              secondary
              type="primary"
              @click="handleSwitchToH264"
            >
              {{ $t("premiere.switchToH264", { res: `${h264CapInfo.h264MaxHeight}p` }) }}
            </n-button>
            <n-button
              size="tiny"
              :type="autoConvertTarget === 'h264_mp4' ? 'success' : 'default'"
              secondary
              @click="handleSetAutoConvert('h264_mp4')"
            >
              {{ $t("premiere.autoConvertH264") }}
              {{ autoConvertTarget === "h264_mp4" ? " ✓" : "" }}
            </n-button>
            <n-button
              size="tiny"
              :type="autoConvertTarget === 'prores_422_lt_mov' ? 'success' : 'default'"
              secondary
              @click="handleSetAutoConvert('prores_422_lt_mov')"
            >
              {{ $t("premiere.autoConvertProres") }}
              {{ autoConvertTarget === "prores_422_lt_mov" ? " ✓" : "" }}
            </n-button>
          </n-flex>
        </n-flex>
      </n-alert>

      <n-radio-group v-model:value="downloadMode" size="small">
        <n-radio-button value="default">{{ $t("common.default") }}</n-radio-button>
        <n-radio-button value="video">{{ $t("detail.videoOnly") }}</n-radio-button>
        <n-radio-button value="audio">{{ $t("detail.audioOnly") }}</n-radio-button>
      </n-radio-group>

      <n-text
        v-if="videoFormatOptions.length === 0 && audioFormatOptions.length === 0"
        depth="3"
        class="auto-format-hint"
      >
        {{ $t("detail.autoFormatHint") }}
      </n-text>

      <n-alert v-if="formatsIncomplete" type="warning" :bordered="false">
        {{ $t("detail.incompleteFormatsHint") }}
      </n-alert>

      <n-alert v-if="isLive" type="info" :bordered="false">
        {{ $t("detail.liveFormatHint") }}
      </n-alert>

      <n-flex v-if="downloadMode !== 'audio' && videoFormatOptions.length" align="center" :size="8">
        <n-text depth="3" style="font-size: 13px; flex-shrink: 0">
          {{ $t("detail.video") }}
        </n-text>
        <n-select
          :value="selectedVideoCodec"
          :options="videoCodecOptions"
          size="small"
          style="width: 118px; flex-shrink: 0"
          :aria-label="$t('detail.codec')"
          @update:value="handleVideoCodecChange"
        />
        <n-select
          v-model:value="selectedVideoFormat"
          :options="videoFormatOptions"
          size="small"
          style="min-width: 0"
        />
      </n-flex>

      <n-flex v-if="downloadMode !== 'video' && audioFormatOptions.length" align="center" :size="8">
        <n-text depth="3" style="font-size: 13px; flex-shrink: 0">
          {{ $t("detail.audio") }}
        </n-text>
        <n-select
          :value="selectedAudioCodec"
          :options="audioCodecOptions"
          size="small"
          style="width: 118px; flex-shrink: 0"
          :aria-label="$t('detail.codec')"
          @update:value="handleAudioCodecChange"
        />
        <n-select
          v-model:value="selectedAudioFormat"
          :options="audioFormatOptions"
          size="small"
          style="min-width: 0"
        />
      </n-flex>
    </n-flex>
  </n-card>
</template>

<style scoped>
.auto-format-hint {
  font-size: 13px;
  text-wrap: pretty;
}
</style>
