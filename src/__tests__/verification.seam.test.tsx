import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, expect, it } from "vitest";
import { App } from "../App";
import { DEFAULT_SETTINGS, type LearningProgress } from "../core/contracts";
import { createBuiltinGoCourseFixture, FakeCoreClient } from "./fakeCoreClient";

beforeEach(() => localStorage.clear());
async function open() {
  const course = createBuiltinGoCourseFixture();
  const chapter = course.chapters.find(c => c.chapterId === "ex-0")!;
  chapter.threeStep!.step3Checks.unshift({ id: "auto", title: "API contract", kind: "commandTest", required: true, cwd: null, command: "go test -json -race ./...", method: null, url: null });
  const client = new FakeCoreClient({ settings: { ...DEFAULT_SETTINGS, locale: "en-US" }, builtinCourses: [course], pickedPaths: { workspaceDir: "/work/checks" } });
  client.writeDiskFile("/work/checks", "main.go", "package main\n");
  const user = userEvent.setup(); render(<App client={client} />);
  await user.click(await screen.findByRole("button", { name: "Start Built-in Go / GoFrame Lab" }));
  await user.click(await screen.findByRole("button", { name: "Choose Workspace" }));
  return { client, user };
}
it("saves drafts before checking and renders structured failures, logs, and sticky completion from Core", async () => {
  const {client,user} = await open();
  await user.click(screen.getByRole("button", {name:"Files"}));
  await user.click(await screen.findByRole("button", {name:"main.go"}));
  fireEvent.change(await screen.findByRole("textbox", {name:"Edit main.go"}), {target:{value:"// saved for checks\n"}});
  await user.click(screen.getByRole("button",{name:"Run checks"}));
  const request=client.commandLog.find(c=>c.type==="runChecks")!;
  expect(request.payload).toMatchObject({chapterId:"ex-0",drafts:[{path:"main.go",text:"// saved for checks\n"}]});
  const progress: LearningProgress={workspaceId:client.progress!.workspaceId,completedExercises:1,totalExercises:10,runningChapterId:null,chapters:{"ex-0":{completionState:"completed",selfChecks:{},lastCheckResults:[{checkId:"auto",status:"failed",durationMs:42,diffs:[{label:"status",expected:"201",actual:"200"}],rawOutput:"contract mismatch",testCases:[],error:null}]}}};
  act(()=>client.emitEvent({type:"progressUpdated",payload:progress}));
  const panel=await screen.findByRole("tabpanel",{name:/Check results/});
  expect(within(panel).getByText("201")).toBeInTheDocument();
  expect(within(panel).getByText("200")).toBeInTheDocument();
  expect(within(panel).getByText("42 ms")).toBeInTheDocument();
  await user.click(within(panel).getByText("Raw output"));
  expect(within(panel).getByText("contract mismatch")).toBeVisible();
  expect(screen.getByText("Completed",{selector:".verification-completed"})).toBeInTheDocument();
  expect(screen.queryByRole("tab",{name:"Terminal 1"})).not.toBeInTheDocument();
});
it("aborts checks and focuses the disk-conflict banner while preserving the draft",async()=>{
  const {client,user}=await open();
  await user.click(screen.getByRole("button",{name:"Files"}));
  await user.click(await screen.findByRole("button",{name:"main.go"}));
  const editor=await screen.findByRole("textbox",{name:"Edit main.go"});
  fireEvent.change(editor,{target:{value:"// draft"}});
  act(()=>client.writeDiskFile("/work/checks","main.go","// external"));
  await user.click(screen.getByRole("button",{name:"Run checks"}));
  await waitFor(()=>expect(screen.getByRole("region",{name:"Changed on disk"})).toHaveFocus());
  expect(editor).toHaveValue("// draft"); expect(client.progress?.runningChapterId).not.toBe("ex-0");
});
