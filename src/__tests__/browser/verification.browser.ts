import { expect, test } from "@playwright/test";

test("check results preserve the editor, compare failures, and expose local progress actions", async ({page}) => {
  const errors: string[]=[]; page.on("pageerror",e=>errors.push(e.message));
  await page.goto("/src/__tests__/browser/workspace.html");
  await page.getByRole("button",{name:"Start Built-in Go / GoFrame Lab"}).click();
  await page.getByRole("button",{name:"Choose Workspace"}).click();
  await page.getByRole("button",{name:"Next Chapter"}).click();
  await page.getByRole("button",{name:"Run checks"}).click();
  await page.evaluate(()=>window.workspaceSmoke.setCheckProgress({workspaceId:1,completedExercises:1,totalExercises:10,runningChapterId:null,chapters:{"ex-1":{completionState:"completed",selfChecks:{},lastCheckResults:[{checkId:"v1",status:"failed",durationMs:42,diffs:[{label:"status",expected:"201",actual:"200"}],rawOutput:"HTTP 200\ncontract mismatch",testCases:[],error:null}]}}}));
  const results=page.getByRole("tabpanel",{name:/Check results/});
  await expect(results.getByText("Expected",{exact:true})).toBeVisible();
  await expect(results.getByText("201",{exact:true})).toBeVisible();
  await expect(results.getByText("200",{exact:true})).toBeVisible();
  await results.getByText("Raw output").click();
  await expect(results.getByText(/HTTP 200/)).toBeVisible();
  await expect(page.locator(".verification-completed")).toHaveText("Completed");
  await expect(page.getByRole("button",{name:"Export learning record"})).toBeVisible();
  await page.getByRole("button",{name:"Reset progress",exact:true}).click();
  await expect(page.getByRole("button",{name:"Cancel",exact:true})).toBeFocused();
  await page.keyboard.press("Escape");
  await page.screenshot({path:"/tmp/codelab-verification-1440.png"});
  await page.setViewportSize({width:1024,height:720});
  await expect(results.getByText("201",{exact:true})).toBeVisible();
  await page.screenshot({path:"/tmp/codelab-verification-1024.png"});
  expect(errors).toEqual([]);
});
