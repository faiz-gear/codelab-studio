import {
  useCallback,
  useEffect,
  useId,
  useLayoutEffect,
  useRef,
  useState,
  type CSSProperties,
  type KeyboardEvent,
} from "react";
import { CheckIcon, ChevronDownIcon } from "./Icons";

export interface SelectOption<T extends string = string> {
  value: T;
  label: string;
  description?: string;
  disabled?: boolean;
}

export interface SelectProps<T extends string = string> {
  id?: string;
  value: T;
  options: readonly SelectOption<T>[];
  onChange: (value: T) => void;
  disabled?: boolean;
  size?: "sm" | "md";
  mono?: boolean;
  placeholder?: string;
  className?: string;
  menuClassName?: string;
  "aria-label"?: string;
  "aria-labelledby"?: string;
  "aria-describedby"?: string;
  "data-testid"?: string;
}

interface PopoverElement extends HTMLDivElement {
  showPopover: () => void;
  hidePopover: () => void;
}

function supportsPopover(el: HTMLDivElement | null): el is PopoverElement {
  return Boolean(
    el &&
      typeof (el as Partial<PopoverElement>).showPopover === "function" &&
      typeof (el as Partial<PopoverElement>).hidePopover === "function"
  );
}

function findNextEnabledIndex<T extends string>(
  options: readonly SelectOption<T>[],
  fromIndex: number,
  direction: 1 | -1
): number {
  const count = options.length;
  if (count === 0) return -1;
  for (let step = 1; step <= count; step += 1) {
    const candidate = (fromIndex + direction * step + count) % count;
    if (!options[candidate]?.disabled) {
      return candidate;
    }
  }
  return -1;
}

function findEdgeEnabledIndex<T extends string>(
  options: readonly SelectOption<T>[],
  edge: "first" | "last"
): number {
  if (edge === "first") {
    return options.findIndex((option) => !option.disabled);
  }
  for (let index = options.length - 1; index >= 0; index -= 1) {
    if (!options[index]?.disabled) return index;
  }
  return -1;
}

export function Select<T extends string = string>({
  id,
  value,
  options,
  onChange,
  disabled = false,
  size = "md",
  mono = false,
  placeholder,
  className,
  menuClassName,
  "aria-label": ariaLabel,
  "aria-labelledby": ariaLabelledBy,
  "aria-describedby": ariaDescribedBy,
  "data-testid": dataTestId,
}: SelectProps<T>) {
  const generatedId = useId();
  const listboxId = `${generatedId}-listbox`;
  const [open, setOpen] = useState(false);
  const [activeIndex, setActiveIndex] = useState(-1);
  const [menuStyle, setMenuStyle] = useState<CSSProperties>({});

  const rootRef = useRef<HTMLDivElement | null>(null);
  const triggerRef = useRef<HTMLButtonElement | null>(null);
  const menuRef = useRef<HTMLDivElement | null>(null);
  const optionRefs = useRef<(HTMLDivElement | null)[]>([]);
  const typeaheadRef = useRef<{ buffer: string; timer: ReturnType<typeof setTimeout> | null }>({
    buffer: "",
    timer: null,
  });

  const selectedIndex = options.findIndex((option) => option.value === value);
  const selectedOption = selectedIndex >= 0 ? options[selectedIndex] : null;
  const isOpen = open && !disabled;

  const resolveInitialActiveIndex = useCallback(
    (fallbackEdge: "first" | "last" = "first") => {
      if (selectedIndex >= 0 && !options[selectedIndex]?.disabled) {
        return selectedIndex;
      }
      return findEdgeEnabledIndex(options, fallbackEdge);
    },
    [options, selectedIndex]
  );

  const openMenu = useCallback(
    (initialEdge: "selected" | "first" | "last" = "selected") => {
      if (disabled) return;
      const nextIndex =
        initialEdge === "selected"
          ? resolveInitialActiveIndex("first")
          : findEdgeEnabledIndex(options, initialEdge);
      setActiveIndex(nextIndex);
      setOpen(true);
    },
    [disabled, options, resolveInitialActiveIndex]
  );

  const closeMenu = useCallback((restoreFocus = false) => {
    setOpen(false);
    if (restoreFocus) {
      queueMicrotask(() => triggerRef.current?.focus());
    }
  }, []);

  const commitOption = useCallback(
    (option: SelectOption<T> | undefined) => {
      if (!option || option.disabled) return;
      setOpen(false);
      if (option.value !== value) {
        onChange(option.value);
      }
      queueMicrotask(() => triggerRef.current?.focus());
    },
    [onChange, value]
  );

  const positionMenu = useCallback(() => {
    const trigger = triggerRef.current;
    const menu = menuRef.current;
    if (!trigger || !menu) return;

    const triggerRect = trigger.getBoundingClientRect();
    const viewportW = window.innerWidth || 1024;
    const viewportH = window.innerHeight || 720;
    const margin = 8;
    const gap = 6;

    const minWidth = Math.min(Math.max(triggerRect.width, 132), viewportW - margin * 2);
    const maxWidth = Math.min(Math.max(triggerRect.width, 320), viewportW - margin * 2);

    const spaceBelow = Math.max(0, viewportH - triggerRect.bottom - gap - margin);
    const spaceAbove = Math.max(0, triggerRect.top - gap - margin);
    const measuredHeight = Math.min(menu.scrollHeight || 220, 280);
    const openAbove = spaceBelow < Math.min(measuredHeight, 180) && spaceAbove > spaceBelow;
    const availableHeight = openAbove ? spaceAbove : spaceBelow;
    const maxHeight = Math.max(88, Math.min(280, Math.floor(availableHeight)));
    const effectiveHeight = Math.min(measuredHeight, maxHeight);

    const top = openAbove
      ? Math.max(margin, triggerRect.top - gap - effectiveHeight)
      : Math.min(viewportH - margin - effectiveHeight, triggerRect.bottom + gap);

    const renderedWidth = Math.min(Math.max(menu.offsetWidth || minWidth, minWidth), maxWidth);
    const left = Math.max(margin, Math.min(triggerRect.left, viewportW - margin - renderedWidth));

    setMenuStyle({
      top: `${Math.round(top)}px`,
      left: `${Math.round(left)}px`,
      minWidth: `${Math.round(minWidth)}px`,
      maxWidth: `${Math.round(maxWidth)}px`,
      maxHeight: `${Math.round(maxHeight)}px`,
    });
  }, []);

  useLayoutEffect(() => {
    if (!isOpen) return;
    const menu = menuRef.current;
    if (supportsPopover(menu)) {
      try {
        menu.showPopover();
      } catch {
        // Ignore if already open or detached.
      }
    }
    positionMenu();
    return () => {
      if (supportsPopover(menu)) {
        try {
          menu.hidePopover();
        } catch {
          // Ignore if already closed or detached.
        }
      }
    };
  }, [isOpen, positionMenu]);

  useEffect(() => {
    if (!isOpen) return;
    const activeNode = optionRefs.current[activeIndex];
    if (activeNode && typeof activeNode.scrollIntoView === "function") {
      activeNode.scrollIntoView({ block: "nearest" });
    }
  }, [activeIndex, isOpen]);

  useEffect(() => {
    if (!isOpen) return;

    const handlePointerDown = (event: PointerEvent | MouseEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) {
        closeMenu(false);
      }
    };

    const handleFocusIn = (event: FocusEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) {
        closeMenu(false);
      }
    };

    // Capture Escape before window/dialog listeners so closing the menu never dismisses the parent modal.
    const handleCaptureKeyDown = (event: globalThis.KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        event.stopPropagation();
        closeMenu(true);
      }
    };

    const handleReposition = (event: Event) => {
      if (event.type === "scroll" && menuRef.current?.contains(event.target as Node)) {
        return;
      }
      positionMenu();
    };

    document.addEventListener("pointerdown", handlePointerDown, true);
    document.addEventListener("focusin", handleFocusIn, true);
    window.addEventListener("keydown", handleCaptureKeyDown, true);
    window.addEventListener("resize", handleReposition);
    window.addEventListener("scroll", handleReposition, true);

    return () => {
      document.removeEventListener("pointerdown", handlePointerDown, true);
      document.removeEventListener("focusin", handleFocusIn, true);
      window.removeEventListener("keydown", handleCaptureKeyDown, true);
      window.removeEventListener("resize", handleReposition);
      window.removeEventListener("scroll", handleReposition, true);
    };
  }, [closeMenu, isOpen, positionMenu]);

  useEffect(() => {
    const typeahead = typeaheadRef.current;
    return () => {
      if (typeahead.timer !== null) {
        clearTimeout(typeahead.timer);
      }
    };
  }, []);

  const handleKeyDown = (event: KeyboardEvent<HTMLButtonElement>) => {
    if (disabled) return;

    if (!isOpen) {
      if (event.key === "ArrowDown" || event.key === "Enter" || event.key === " ") {
        event.preventDefault();
        openMenu("selected");
      } else if (event.key === "ArrowUp") {
        event.preventDefault();
        openMenu("selected");
      } else if (event.key === "Home") {
        event.preventDefault();
        openMenu("first");
      } else if (event.key === "End") {
        event.preventDefault();
        openMenu("last");
      }
      return;
    }

    if (event.key === "ArrowDown") {
      event.preventDefault();
      setActiveIndex((current) =>
        findNextEnabledIndex(options, current < 0 ? -1 : current, 1)
      );
      return;
    }

    if (event.key === "ArrowUp") {
      event.preventDefault();
      setActiveIndex((current) =>
        findNextEnabledIndex(options, current < 0 ? 0 : current, -1)
      );
      return;
    }

    if (event.key === "Home") {
      event.preventDefault();
      setActiveIndex(findEdgeEnabledIndex(options, "first"));
      return;
    }

    if (event.key === "End") {
      event.preventDefault();
      setActiveIndex(findEdgeEnabledIndex(options, "last"));
      return;
    }

    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      if (activeIndex >= 0) {
        commitOption(options[activeIndex]);
      } else {
        closeMenu(true);
      }
      return;
    }

    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      closeMenu(true);
      return;
    }

    if (event.key === "Tab") {
      closeMenu(false);
      return;
    }

    if (event.key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey) {
      const store = typeaheadRef.current;
      if (store.timer !== null) clearTimeout(store.timer);
      store.buffer = (store.buffer + event.key).toLowerCase();
      store.timer = setTimeout(() => {
        store.buffer = "";
        store.timer = null;
      }, 500);
      const start = activeIndex >= 0 ? activeIndex : 0;
      const count = options.length;
      for (let offset = 0; offset < count; offset += 1) {
        const idx = (start + offset) % count;
        const opt = options[idx];
        if (!opt.disabled && opt.label.toLowerCase().startsWith(store.buffer)) {
          setActiveIndex(idx);
          break;
        }
      }
    }
  };

  const activeOptionId =
    isOpen && activeIndex >= 0 && options[activeIndex]
      ? `${generatedId}-opt-${activeIndex}`
      : undefined;

  const displayLabel = selectedOption?.label ?? placeholder ?? "";
  const isPlaceholder = !selectedOption && Boolean(placeholder);

  return (
    <div
      ref={rootRef}
      className={`arc-select${size === "sm" ? " arc-select--sm" : ""}${className ? ` ${className}` : ""}`}
    >
      <button
        ref={triggerRef}
        id={id}
        type="button"
        role="combobox"
        className={`arc-select-trigger${size === "sm" ? " arc-select-trigger--sm" : ""}${mono ? " mono" : ""}`}
        aria-haspopup="listbox"
        aria-expanded={isOpen}
        aria-controls={isOpen ? listboxId : undefined}
        aria-activedescendant={activeOptionId}
        aria-label={ariaLabel}
        aria-labelledby={ariaLabelledBy}
        aria-describedby={ariaDescribedBy}
        data-value={value}
        data-testid={dataTestId}
        disabled={disabled}
        onClick={() => {
          if (isOpen) closeMenu(false);
          else openMenu("selected");
        }}
        onKeyDown={handleKeyDown}
      >
        <span className={`arc-select-value${isPlaceholder ? " is-placeholder" : ""}`}>
          {displayLabel}
        </span>
        <span className="arc-select-chevron" aria-hidden="true">
          <ChevronDownIcon size={size === "sm" ? 12 : 14} />
        </span>
      </button>

      {isOpen && (
        <div
          ref={menuRef}
          id={listboxId}
          popover="manual"
          role="listbox"
          className={`arc-select-menu${size === "sm" ? " arc-select-menu--sm" : ""}${mono ? " mono" : ""}${menuClassName ? ` ${menuClassName}` : ""}`}
          style={menuStyle}
          aria-label={ariaLabel}
          aria-labelledby={ariaLabel ? undefined : ariaLabelledBy}
        >
          {options.map((option, index) => {
            const isSelected = option.value === value;
            const isActive = index === activeIndex;
            return (
              <div
                key={option.value}
                ref={(el) => {
                  optionRefs.current[index] = el;
                }}
                id={`${generatedId}-opt-${index}`}
                role="option"
                aria-selected={isSelected}
                aria-disabled={option.disabled || undefined}
                data-value={option.value}
                data-active={isActive ? "true" : undefined}
                className={`arc-select-option${isSelected ? " is-selected" : ""}${isActive ? " is-active" : ""}${option.disabled ? " is-disabled" : ""}`}
                onMouseEnter={() => {
                  if (!option.disabled) setActiveIndex(index);
                }}
                onMouseDown={(event) => {
                  event.preventDefault();
                }}
                onClick={(event) => {
                  event.preventDefault();
                  event.stopPropagation();
                  commitOption(option);
                }}
              >
                <span className="arc-select-option-copy">
                  <span className="arc-select-option-label">{option.label}</span>
                  {option.description ? (
                    <span className="arc-select-option-desc">{option.description}</span>
                  ) : null}
                </span>
                <span className="arc-select-check" aria-hidden="true">
                  {isSelected ? <CheckIcon size={13} /> : null}
                </span>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
