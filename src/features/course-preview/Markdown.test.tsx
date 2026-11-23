import { renderToStaticMarkup } from "react-dom/server";
import { expect, it } from "vitest";
import { Markdown } from "./Markdown";

it("renders structured HTML while keeping raw markup and image paths inert", () => {
  const html = renderToStaticMarkup(
    <Markdown nodes={[
      { kind: "heading", level: 3, id: "verify", children: [{ kind: "text", value: "Verify" }] },
      { kind: "paragraph", children: [
        { kind: "text", value: "<script>marker</script>" },
        { kind: "link", href: "#verify", title: null, children: [{ kind: "text", value: "Review" }] },
        { kind: "image", src: "https://example.test/diagram.png", alt: "Course diagram", title: null },
      ] },
    ]} />,
  );
  expect(html).toContain('<h3 id="manual-verify">Verify</h3>');
  expect(html).toContain('<a href="#manual-verify">Review</a>');
  expect(html).toContain("&lt;script&gt;marker&lt;/script&gt;");
  expect(html).toContain("Course diagram");
  expect(html).not.toMatch(/<(script|img)\b/);
});
