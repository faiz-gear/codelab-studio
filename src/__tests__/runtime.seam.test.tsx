import { act, fireEvent, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, it } from "vitest";
import { App } from "../App";
import type { PreflightReport } from "../core/contracts";
import { FakeCoreClient } from "./fakeCoreClient";

const initialReport: PreflightReport = {
  revision: 1,
  checked: true,
  probing: false,
  installingCount: 0,
  installDir: "/codelab/runtimes",
  languages: [
    {
      language: "Go",
      status: "ok",
      system: [
        {
          tool: "go",
          status: "ok",
          value: "1.23.12",
          executable: "/usr/local/bin/go",
          problem: null,
        },
      ],
      lsp: {
        tool: "gopls",
        status: "missing",
        value: null,
        executable: null,
        problem: null,
      },
      managed: null,
      selected: { source: "system", tool: "go" },
      preferManaged: false,
      canInstall: false,
      install: null,
    },
  ],
  tools: [],
  goProxy: {
    tool: "goProxy",
    status: "ok",
    value: "https://proxy.golang.org,direct",
    executable: null,
    problem: null,
  },
};

beforeEach(() => {
  window.localStorage.clear();
  // JSDOM omits the native modal dialog methods.
  Object.defineProperties(HTMLDialogElement.prototype, {
    showModal: {
      configurable: true,
      value(this: HTMLDialogElement) {
        this.setAttribute("open", "");
      },
    },
    close: {
      configurable: true,
      value(this: HTMLDialogElement) {
        this.removeAttribute("open");
      },
    },
  });
});

it("shows the same global install progress in Settings and Preflight after switching Courses", async () => {
  const user = userEvent.setup();
  const client = new FakeCoreClient({
    settings: { locale: "en-US" },
    preflight: initialReport,
  });
  render(<App client={client} />);

  const trigger = await screen.findByRole("button", { name: /Preflight/ });
  await user.click(trigger);
  const popover = () =>
    within(screen.getByRole("dialog", { name: "Preflight" }));
  expect(
    popover().getByText("https://proxy.golang.org,direct"),
  ).toBeInTheDocument();
  expect(popover().getByText("gopls")).toBeInTheDocument();
  expect(
    popover().queryByRole("button", { name: /Install Managed Runtime/ }),
  ).toBeNull();

  await user.click(popover().getByRole("button", { name: "Manage runtimes" }));
  const settings = within(screen.getByRole("dialog", { name: "Settings" }));
  expect(
    settings.getByRole("tab", { name: "4-Language Runtimes" }),
  ).toHaveAttribute("aria-selected", "true");
  await user.click(
    settings.getByRole("checkbox", { name: "Prefer Managed Runtime for Go" }),
  );
  expect(client.commandLog).toContainEqual({
    type: "setRuntimePreference",
    payload: { language: "Go", preferManaged: true },
  });
  await user.click(
    settings.getByRole("button", { name: "Install Managed Runtime for Go" }),
  );
  expect(client.commandLog).toContainEqual({
    type: "installManagedRuntime",
    payload: { language: "Go" },
  });
  expect(settings.getByText("Preparing installation")).toBeInTheDocument();
  fireEvent(
    screen.getByRole("dialog", { name: "Settings" }),
    new Event("cancel", { cancelable: true }),
  );
  await act(async () => {});
  expect(screen.queryByRole("dialog", { name: "Settings" })).toBeNull();
  expect(trigger).toHaveFocus();

  await user.click(
    within(screen.getByTestId("course-grid")).getByRole("button", {
      name: /Go.*GoFrame/,
    }),
  );
  expect(client.commandLog).toContainEqual({
    type: "openCourse",
    payload: { kind: "builtin", courseId: "go-goframe-practice" },
  });

  const downloading: PreflightReport = {
    ...initialReport,
    revision: 10,
    installingCount: 1,
    languages: [
      {
        ...initialReport.languages[0],
        preferManaged: true,
        install: {
          phase: "downloading",
          artifact: "go.tar.gz",
          receivedBytes: 4,
          totalBytes: 10,
        },
      },
    ],
  };
  act(() =>
    client.emitEvent({ type: "preflightUpdated", payload: downloading }),
  );
  await user.click(trigger);
  expect(
    popover().getByRole("progressbar", { name: "Go installation" }),
  ).toHaveValue(4);
  await user.click(popover().getByRole("button", { name: "Manage runtimes" }));
  const reopened = within(screen.getByRole("dialog", { name: "Settings" }));
  expect(
    reopened.getByRole("progressbar", { name: "Go installation" }),
  ).toHaveValue(4);

  act(() =>
    client.emitEvent({
      type: "preflightUpdated",
      payload: {
        ...downloading,
        revision: 11,
        installingCount: 0,
        languages: [
          {
            ...downloading.languages[0],
            selected: { source: "managed" },
            managed: {
              version: "1.23.12",
              executable: "/codelab/runtimes/go/bin/go",
              testExecutor: "/codelab/runtimes/go/bin/go",
              uv: null,
            },
            install: { phase: "installed" },
          },
        ],
      },
    }),
  );
  expect(reopened.getByText("Managed Runtime installed")).toBeInTheDocument();
  expect(
    reopened.getByText("Managed Runtime", { exact: true }),
  ).toBeInTheDocument();
});

it("rechecks Preflight and offers a localized retry after installation fails", async () => {
  const user = userEvent.setup();
  const go = initialReport.languages[0];
  const client = new FakeCoreClient({
    preflight: {
      ...initialReport,
      languages: [
        {
          ...go,
          status: "warn",
          system: [
            {
              ...go.system[0],
              status: "warn",
              value: "1.21.0",
              problem: "versionTooOld",
            },
          ],
          selected: null,
          canInstall: true,
        },
      ],
    },
  });
  render(<App client={client} />);
  await user.click(screen.getByRole("button", { name: "设置" }));
  const settings = within(screen.getByRole("dialog", { name: "设置" }));
  expect(settings.getByText("版本过低")).toBeInTheDocument();
  await user.click(settings.getByRole("button", { name: "重新探测" }));
  expect(
    client.commandLog.filter((command) => command.type === "refreshPreflight"),
  ).toHaveLength(2);
  await user.click(
    settings.getByRole("button", { name: "为 Go 安装 Managed Runtime" }),
  );
  act(() =>
    client.emitEvent({
      type: "preflightUpdated",
      payload: {
        ...initialReport,
        revision: 12,
        languages: [
          {
            ...go,
            status: "missing",
            selected: null,
            canInstall: true,
            install: {
              phase: "failed",
              error: { code: "RUNTIME_DOWNLOAD_FAILED" },
            },
          },
        ],
      },
    }),
  );
  expect(settings.getByRole("alert")).toHaveTextContent("运行时下载失败");
  await user.click(
    settings.getByRole("button", { name: "重试安装 Go Managed Runtime" }),
  );
  expect(settings.getByText("正在准备安装")).toBeInTheDocument();
});
