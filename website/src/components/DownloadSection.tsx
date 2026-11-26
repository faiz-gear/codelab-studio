import { useState } from "react";
import {
  CHECKSUMS_URL,
  COPY,
  INSTALLERS,
  RELEASE_TAG_URL,
  RELEASE_VERSION,
  REPO_URL,
  type Locale,
  type PlatformId,
} from "../data/content";
import {
  BookIcon,
  CheckIcon,
  CopyIcon,
  DownloadIcon,
  ExternalIcon,
  PlatformIcon,
  ShieldIcon,
} from "./Icons";

interface DownloadSectionProps {
  locale: Locale;
  detectedPlatform: PlatformId;
  selectedPlatform: PlatformId;
  onSelectPlatform: (platform: PlatformId) => void;
}

const PLATFORM_ORDER: PlatformId[] = ["macos", "windows", "linux"];

export function DownloadSection({
  locale,
  detectedPlatform,
  selectedPlatform,
  onSelectPlatform,
}: DownloadSectionProps) {
  const t = COPY[locale].downloadSection;
  const asset = INSTALLERS[selectedPlatform];
  const platformCopy = t.platforms[selectedPlatform];
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(asset.verifyCmd);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1800);
    } catch {
      setCopied(false);
    }
  };

  const authorSpecUrl =
    locale === "zh-CN"
      ? `${REPO_URL}/blob/${RELEASE_VERSION}/README.zh-CN.md#course-pack-目录格式`
      : `${REPO_URL}/blob/${RELEASE_VERSION}/README.md#course-pack-format`;

  return (
    <section id="download" className="scroll-mt-20 border-t-2 border-ink py-14 lg:py-20">
      <div className="max-w-2xl">
        <h2 className="font-marker text-3xl leading-tight tracking-[-0.015em] text-ink sm:text-4xl">
          {t.heading}
        </h2>
        <p className="mt-3 text-base leading-relaxed text-ink-soft">
          {t.lead}
        </p>
      </div>

      {/* Platform Switcher Tabs */}
      <div className="mt-8 flex flex-wrap gap-3">
        {PLATFORM_ORDER.map((id) => {
          const isSelected = id === selectedPlatform;
          const isDetected = id === detectedPlatform;
          return (
            <button
              key={id}
              type="button"
              onClick={() => onSelectPlatform(id)}
              className={`inline-flex items-center gap-2 rounded-[12px] border-2 border-ink px-4 py-2.5 text-sm font-bold transition-colors ${
                isSelected
                  ? "bg-hl-yellow text-ink"
                  : "bg-paper-bright text-ink hover:bg-paper-tint"
              }`}
            >
              <PlatformIcon platform={id} size={17} />
              <span className="whitespace-nowrap">{t.platforms[id].short}</span>
              <span className="whitespace-nowrap font-mono text-xs font-medium text-ink-soft tabular">
                {INSTALLERS[id].sizeLabel}
              </span>
              {isDetected && (
                <span className="whitespace-nowrap rounded-full bg-paper-bright px-2 py-0.5 text-[10.5px] font-bold text-hl-teal-deep">
                  {t.detectedTag}
                </span>
              )}
            </button>
          );
        })}
      </div>

      {/* Single Focal Download Plate (zero nested bordered boxes inside; halftone mat cleanly behind opaque inner card) */}
      <div className="zine-plate-focal mt-5">
        <div className="zine-plate-focal-inner">
          <div className="grid grid-cols-1 lg:grid-cols-12">
          {/* Left: Direct Installer Action + Checksum Band */}
          <div className="flex flex-col justify-between border-b-2 border-ink bg-paper-bright lg:col-span-7 lg:border-b-0 lg:border-r-2">
            <div className="p-6 sm:p-7">
              <div className="flex items-start gap-3.5">
                <span className="inline-flex h-10 w-10 shrink-0 items-center justify-center rounded-[10px] border-2 border-ink bg-hl-yellow">
                  <PlatformIcon platform={selectedPlatform} size={20} />
                </span>
                <div className="min-w-0">
                  <h3 className="text-lg font-extrabold text-ink">
                    {platformCopy.name}
                  </h3>
                  <p className="mt-0.5 break-all font-mono text-xs text-ink-soft tabular">
                    {asset.fileName} · {asset.sizeLabel} · {RELEASE_VERSION}
                  </p>
                </div>
              </div>

              <div className="mt-6 flex flex-wrap items-center gap-3">
                <a
                  href={asset.downloadUrl}
                  className="zine-btn-primary px-6 py-3.5 text-base"
                >
                  <DownloadIcon size={18} />
                  <span>{t.downloadCta}</span>
                </a>
                <a
                  href={CHECKSUMS_URL}
                  target="_blank"
                  rel="noreferrer"
                  className="zine-btn-secondary px-4 py-3.5 text-sm"
                >
                  <span>{t.checksumsLink}</span>
                  <ExternalIcon size={14} />
                </a>
              </div>
            </div>

            {/* Borderless tonal SHA-256 verification footer band */}
            <div className="border-t-2 border-ink/20 bg-paper px-6 py-4 sm:px-7">
              <div className="flex items-center justify-between gap-2 font-mono text-[11px] font-bold text-ink-soft">
                <span>SHA-256</span>
                <button
                  type="button"
                  onClick={handleCopy}
                  className="inline-flex items-center gap-1 text-hl-teal-deep hover:underline"
                >
                  {copied ? <CheckIcon size={12} /> : <CopyIcon size={12} />}
                  {copied ? t.copiedCmd : t.copyCmd}
                </button>
              </div>
              <code className="mt-1.5 block overflow-x-auto font-mono text-xs text-ink">
                {asset.verifyCmd}
              </code>
            </div>
          </div>

          {/* Right: Transparent First-Launch Guidance (GitHub Releases links only) */}
          <div className="flex flex-col justify-between gap-5 bg-paper p-6 sm:p-7 lg:col-span-5">
            <div>
              <div className="flex items-center gap-2 text-sm font-bold text-ink">
                <ShieldIcon size={17} className="shrink-0 text-hl-pink-deep" />
                <h4>{t.firstLaunchTitle}</h4>
              </div>
              <p className="mt-3 font-bold text-ink">
                {platformCopy.noteTitle}
              </p>
              <p className="mt-2 text-sm leading-relaxed text-ink-soft">
                {platformCopy.noteBody}
              </p>
            </div>

            <div className="border-t border-ink/15 pt-4 text-sm">
              <a
                href={RELEASE_TAG_URL}
                target="_blank"
                rel="noreferrer"
                className="inline-flex items-center gap-1.5 font-bold text-hl-teal-deep underline"
              >
                <span>{t.allReleasesLink}</span>
                <ExternalIcon size={13} />
              </a>
            </div>
          </div>
        </div>
        </div>
      </div>

      {/* Open Notebook Author Callout Strip (ink-ruled band, no extra stacked box) */}
      <div className="mt-10 border-y-2 border-ink/25 py-6">
        <div className="flex flex-col gap-4 md:flex-row md:items-center md:justify-between">
          <div className="max-w-2xl">
            <div className="flex items-center gap-2 font-bold text-ink">
              <BookIcon size={17} className="shrink-0 text-hl-teal-deep" />
              <h3 className="font-marker text-lg text-ink">{t.authorTitle}</h3>
            </div>
            <p className="mt-1.5 text-sm leading-relaxed text-ink-soft">
              {t.authorBody}
            </p>
          </div>
          <a
            href={authorSpecUrl}
            target="_blank"
            rel="noreferrer"
            className="zine-btn-secondary shrink-0 px-4 py-2.5 text-sm"
          >
            <span>{t.authorLink}</span>
            <ExternalIcon size={14} />
          </a>
        </div>
      </div>
    </section>
  );
}
