import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useEffect, useState } from "react";
import { describe, expect, it, vi } from "vitest";
import { Select, type SelectOption } from "../components/Select";
import {
  computeBottomBounds,
  computeTreeBounds,
  resolveHorizontalWorkbenchWidths,
} from "../components/workbenchLayout";

const OPTIONS: readonly SelectOption[] = [
  { value: "alpha", label: "Alpha" },
  { value: "beta", label: "Beta", disabled: true },
  { value: "gamma", label: "Gamma" },
  { value: "delta", label: "Delta" },
];

function SelectHarness({
  initial = "alpha",
  disabled = false,
  onOuterEscape,
}: {
  initial?: string;
  disabled?: boolean;
  onOuterEscape?: () => void;
}) {
  const [value, setValue] = useState(initial);
  useEffect(() => {
    if (!onOuterEscape) return;
    const handler = (event: KeyboardEvent) => {
      if (event.key === "Escape") onOuterEscape();
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [onOuterEscape]);

  return (
    <div>
      <label htmlFor="demo-select">Demo field</label>
      <Select
        id="demo-select"
        value={value}
        disabled={disabled}
        placeholder="Choose one"
        options={OPTIONS}
        onChange={setValue}
      />
      <button type="button">Next control</button>
    </div>
  );
}

describe("Arc UI Select", () => {
  it("supports mouse open/select/close, accessible names, and skips disabled options", async () => {
    const user = userEvent.setup();
    render(<SelectHarness />);

    const combo = screen.getByRole("combobox", { name: "Demo field" });
    expect(combo).toHaveTextContent("Alpha");
    expect(combo).toHaveAttribute("aria-expanded", "false");

    await user.click(combo);
    expect(combo).toHaveAttribute("aria-expanded", "true");
    const listbox = screen.getByRole("listbox");
    expect(listbox).toBeVisible();

    const disabledOpt = screen.getByRole("option", { name: "Beta" });
    expect(disabledOpt).toHaveAttribute("aria-disabled", "true");
    await user.click(disabledOpt);
    // Clicking disabled option leaves menu open and selection unchanged.
    expect(combo).toHaveAttribute("aria-expanded", "true");
    expect(combo).toHaveTextContent("Alpha");

    await user.click(screen.getByRole("option", { name: "Gamma" }));
    expect(combo).toHaveAttribute("aria-expanded", "false");
    expect(combo).toHaveTextContent("Gamma");
    expect(combo).toHaveFocus();
  });

  it("supports keyboard navigation (ArrowDown/ArrowUp/Home/End/Enter/Space/Escape/Tab) and shields Escape from parent modal listeners", async () => {
    const user = userEvent.setup();
    const onOuterEscape = vi.fn();
    render(<SelectHarness onOuterEscape={onOuterEscape} />);

    const combo = screen.getByRole("combobox", { name: "Demo field" });
    combo.focus();

    // Open with ArrowDown, move past disabled Beta to Gamma, commit with Enter.
    await user.keyboard("{ArrowDown}");
    expect(combo).toHaveAttribute("aria-expanded", "true");
    await user.keyboard("{ArrowDown}");
    await user.keyboard("{Enter}");
    expect(combo).toHaveTextContent("Gamma");
    expect(combo).toHaveAttribute("aria-expanded", "false");

    // Open with Space, jump to End (Delta) then Home (Alpha), cancel with Escape without closing outer modal.
    await user.keyboard(" ");
    expect(combo).toHaveAttribute("aria-expanded", "true");
    await user.keyboard("{End}");
    expect(screen.getByRole("option", { name: "Delta" })).toHaveAttribute("data-active", "true");
    await user.keyboard("{Home}");
    expect(screen.getByRole("option", { name: "Alpha" })).toHaveAttribute("data-active", "true");
    await user.keyboard("{Escape}");
    expect(combo).toHaveAttribute("aria-expanded", "false");
    expect(combo).toHaveTextContent("Gamma");
    expect(onOuterEscape).not.toHaveBeenCalled();

    // Pressing Escape when menu is closed DOES propagate to outer modal listener.
    await user.keyboard("{Escape}");
    expect(onOuterEscape).toHaveBeenCalledTimes(1);

    // Tab closes an open menu and advances focus to the next focusable element.
    await user.keyboard("{ArrowDown}");
    expect(combo).toHaveAttribute("aria-expanded", "true");
    await user.keyboard("{Tab}");
    expect(combo).toHaveAttribute("aria-expanded", "false");
    expect(screen.getByRole("button", { name: "Next control" })).toHaveFocus();
  });

  it("respects disabled state and placeholder display", async () => {
    const user = userEvent.setup();
    render(<SelectHarness initial="" disabled />);

    const combo = screen.getByRole("combobox", { name: "Demo field" });
    expect(combo).toBeDisabled();
    expect(combo).toHaveTextContent("Choose one");

    await user.click(combo);
    expect(screen.queryByRole("listbox")).toBeNull();
  });
});

describe("Workbench splitter layout bounds", () => {
  it("protects the 560px (>=1440) and 520px (1280..1439) editor floor when docked AI Tutor and Manual resize", () => {
    const wide = resolveHorizontalWorkbenchWidths({
      containerWidth: 1440,
      focusMode: false,
      isTutorDocked: true,
      manualWidthPx: 500,
      tutorWidthPx: 480,
    });
    expect(wide.effectiveTutorWidthPx).toBe(480);
    expect(wide.effectiveManualWidthPx).toBe(400);
    expect(1440 - wide.effectiveManualWidthPx! - wide.effectiveTutorWidthPx).toBeGreaterThanOrEqual(560);

    const mid = resolveHorizontalWorkbenchWidths({
      containerWidth: 1320,
      focusMode: false,
      isTutorDocked: true,
      manualWidthPx: 450,
      tutorWidthPx: 420,
    });
    expect(1320 - mid.effectiveManualWidthPx! - mid.effectiveTutorWidthPx).toBeGreaterThanOrEqual(520);
  });

  it("computes usable terminal and file-tree bounds at 1024x720 and 1440x900", () => {
    const smallBottom = computeBottomBounds(672, 38);
    expect(smallBottom.minPx).toBe(120);
    expect(smallBottom.maximizedPx).toBeGreaterThan(400);
    expect(672 - 38 - smallBottom.maximizedPx).toBeGreaterThanOrEqual(140);

    const largeBottom = computeBottomBounds(852, 38);
    expect(largeBottom.maximizedPx).toBeGreaterThan(550);

    const tree = computeTreeBounds(760);
    expect(tree.minPx).toBe(148);
    expect(tree.maxPx).toBeLessThanOrEqual(420);
    expect(760 - tree.maxPx).toBeGreaterThanOrEqual(320);
  });
});
