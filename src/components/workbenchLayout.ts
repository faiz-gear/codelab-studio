export const DEFAULT_MANUAL_WIDTH_PCT = 38;
export const DOCKED_TUTOR_MANUAL_WIDTH_PCT_WIDE = 32;
export const DOCKED_TUTOR_MANUAL_WIDTH_PCT_NARROW = 28;
export const MIN_MANUAL_WIDTH_PX = 320;
export const DOCKED_TUTOR_MIN_MANUAL_PX = 300;
export const MAX_MANUAL_WIDTH_PCT = 54;
export const FOCUS_RAIL_WIDTH_PX = 52;

export const DEFAULT_TUTOR_WIDTH_PX = 360;
export const MIN_TUTOR_WIDTH_PX = 300;
export const MAX_TUTOR_WIDTH_PX = 560;

export const DEFAULT_TERMINAL_HEIGHT_PX = 212;
export const DEFAULT_VERIFICATION_HEIGHT_PX = 260;
export const MIN_BOTTOM_HEIGHT_PX = 120;
export const MIN_UPPER_WORKSPACE_HEIGHT_PX = 140;
export const MAXIMIZED_TERMINAL_RATIO = 0.72;

export const DEFAULT_TREE_WIDTH_PX = 204;
export const MIN_TREE_WIDTH_PX = 148;
export const MAX_TREE_WIDTH_PX = 420;
export const MIN_EDITOR_CODE_WIDTH_PX = 320;

export interface PaneBounds {
  minPx: number;
  maxPx: number;
}

export function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

export function editorMinWidthFloor(containerWidth: number, isTutorDocked: boolean): number {
  if (!isTutorDocked) return 360;
  return containerWidth >= 1440 ? 560 : 520;
}

export function computeManualBounds(
  containerWidth: number,
  isTutorDocked: boolean,
  tutorWidthPx: number
): PaneBounds {
  const pctMax = Math.round((containerWidth * MAX_MANUAL_WIDTH_PCT) / 100);
  if (isTutorDocked) {
    const floor = editorMinWidthFloor(containerWidth, true);
    const maxPx = Math.max(
      DOCKED_TUTOR_MIN_MANUAL_PX,
      Math.min(pctMax, containerWidth - floor - tutorWidthPx)
    );
    return { minPx: DOCKED_TUTOR_MIN_MANUAL_PX, maxPx };
  }
  const maxPx = Math.max(
    MIN_MANUAL_WIDTH_PX,
    Math.min(pctMax, containerWidth - editorMinWidthFloor(containerWidth, false))
  );
  return { minPx: MIN_MANUAL_WIDTH_PX, maxPx };
}

export function defaultManualWidthPct(
  containerWidth: number,
  isTutorDocked: boolean
): number {
  if (!isTutorDocked) return DEFAULT_MANUAL_WIDTH_PCT;
  return containerWidth >= 1440
    ? DOCKED_TUTOR_MANUAL_WIDTH_PCT_WIDE
    : DOCKED_TUTOR_MANUAL_WIDTH_PCT_NARROW;
}

export function computeDefaultManualWidthPx(
  containerWidth: number,
  isTutorDocked: boolean,
  tutorWidthPx: number
): number {
  const bounds = computeManualBounds(containerWidth, isTutorDocked, tutorWidthPx);
  const rawDefaultPx = Math.round(
    (containerWidth * defaultManualWidthPct(containerWidth, isTutorDocked)) / 100
  );
  return clamp(rawDefaultPx, bounds.minPx, bounds.maxPx);
}

export function computeDockedTutorBounds(
  containerWidth: number,
  focusMode: boolean,
  manualWidthPx: number | null
): PaneBounds {
  const floor = editorMinWidthFloor(containerWidth, true);
  // When manual width is auto (not pinned by the user), CSS max-width lets the manual yield down to its docked minimum.
  const reservedManual = focusMode
    ? FOCUS_RAIL_WIDTH_PX
    : manualWidthPx !== null
      ? Math.max(DOCKED_TUTOR_MIN_MANUAL_PX, manualWidthPx)
      : DOCKED_TUTOR_MIN_MANUAL_PX;
  const maxPx = Math.max(
    MIN_TUTOR_WIDTH_PX,
    Math.min(MAX_TUTOR_WIDTH_PX, containerWidth - floor - reservedManual)
  );
  return { minPx: MIN_TUTOR_WIDTH_PX, maxPx };
}

export function resolveHorizontalWorkbenchWidths({
  containerWidth,
  focusMode,
  isTutorDocked,
  manualWidthPx,
  tutorWidthPx,
}: {
  containerWidth: number;
  focusMode: boolean;
  isTutorDocked: boolean;
  manualWidthPx: number | null;
  tutorWidthPx: number | null;
}): {
  effectiveTutorWidthPx: number;
  effectiveManualWidthPx: number | null;
  displayManualWidthPx: number;
  manualBounds: PaneBounds;
  tutorBounds: PaneBounds;
} {
  const floor = editorMinWidthFloor(containerWidth, isTutorDocked);
  const minManualReserved = focusMode
    ? FOCUS_RAIL_WIDTH_PX
    : DOCKED_TUTOR_MIN_MANUAL_PX;
  const hardMaxTutor = Math.max(
    MIN_TUTOR_WIDTH_PX,
    Math.min(MAX_TUTOR_WIDTH_PX, containerWidth - floor - minManualReserved)
  );
  const effectiveTutorWidthPx = isTutorDocked
    ? clamp(tutorWidthPx ?? DEFAULT_TUTOR_WIDTH_PX, MIN_TUTOR_WIDTH_PX, hardMaxTutor)
    : DEFAULT_TUTOR_WIDTH_PX;

  const manualBounds = computeManualBounds(
    containerWidth,
    isTutorDocked,
    effectiveTutorWidthPx
  );
  const effectiveManualWidthPx =
    manualWidthPx !== null
      ? clamp(manualWidthPx, manualBounds.minPx, manualBounds.maxPx)
      : null;
  const displayManualWidthPx =
    effectiveManualWidthPx ??
    computeDefaultManualWidthPx(
      containerWidth,
      isTutorDocked,
      effectiveTutorWidthPx
    );

  const tutorBounds = computeDockedTutorBounds(
    containerWidth,
    focusMode,
    effectiveManualWidthPx
  );

  return {
    effectiveTutorWidthPx,
    effectiveManualWidthPx,
    displayManualWidthPx,
    manualBounds,
    tutorBounds,
  };
}

export function computeBottomBounds(
  paneCenterHeight: number,
  stripHeight = 38
): PaneBounds & { maximizedPx: number } {
  const minPx = MIN_BOTTOM_HEIGHT_PX;
  const maxPx = Math.max(
    minPx,
    Math.floor(paneCenterHeight - stripHeight - MIN_UPPER_WORKSPACE_HEIGHT_PX)
  );
  const maximizedPx = clamp(
    Math.round(paneCenterHeight * MAXIMIZED_TERMINAL_RATIO),
    minPx,
    maxPx
  );
  return { minPx, maxPx, maximizedPx };
}

export function computeTreeBounds(contentWidth: number): PaneBounds {
  const minPx = MIN_TREE_WIDTH_PX;
  const maxPx = Math.max(
    minPx,
    Math.min(MAX_TREE_WIDTH_PX, Math.floor(contentWidth - MIN_EDITOR_CODE_WIDTH_PX))
  );
  return { minPx, maxPx };
}
