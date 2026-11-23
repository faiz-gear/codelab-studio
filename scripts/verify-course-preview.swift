import AppKit
import ApplicationServices

struct Node {
    let element: AXUIElement
    let role: String
    let text: String
}

func attribute(_ element: AXUIElement, _ name: String) -> CFTypeRef? {
    var value: CFTypeRef?
    return AXUIElementCopyAttributeValue(element, name as CFString, &value) == .success ? value : nil
}

func nodes(_ element: AXUIElement) -> [Node] {
    let role = attribute(element, kAXRoleAttribute) as? String ?? ""
    let strings = [kAXTitleAttribute, kAXDescriptionAttribute, kAXValueAttribute]
        .compactMap { attribute(element, $0) as? String }
        .filter { !$0.isEmpty }
    let text = Array(NSOrderedSet(array: strings)).compactMap { $0 as? String }.joined(separator: " | ")
    let children = attribute(element, kAXChildrenAttribute) as? [AXUIElement] ?? []
    return [Node(element: element, role: role, text: text)] + children.flatMap(nodes)
}

func wait<T>(_ seconds: TimeInterval = 8, for value: () -> T?) -> T? {
    let deadline = Date().addingTimeInterval(seconds)
    repeat {
        if let result = value() { return result }
        Thread.sleep(forTimeInterval: 0.08)
    } while Date() < deadline
    return nil
}

func postKey(_ key: CGKeyCode, flags: CGEventFlags = [], text: String? = nil, to pid: pid_t) {
    for down in [true, false] {
        guard let event = CGEvent(keyboardEventSource: nil, virtualKey: key, keyDown: down) else { continue }
        event.flags = flags
        if let text {
            let units = Array(text.utf16)
            event.keyboardSetUnicodeString(stringLength: units.count, unicodeString: units)
        }
        guard NSWorkspace.shared.frontmostApplication?.processIdentifier == pid else { return }
        event.post(tap: .cghidEventTap)
    }
}

func press(_ node: Node) -> Bool {
    AXUIElementPerformAction(node.element, kAXPressAction as CFString) == .success
}

func writeSnapshot(_ snapshot: [Node], to path: URL) throws {
    let lines = snapshot.map { "\($0.role)\t\($0.text.replacingOccurrences(of: "\n", with: "\\n"))" }
    try lines.joined(separator: "\n").write(to: path, atomically: true, encoding: .utf8)
}

func require(_ condition: Bool, _ message: String) throws {
    if !condition { throw NSError(domain: message, code: 1) }
}

func openMarkdown(_ path: String, app: AXUIElement, pid: pid_t) throws -> Date {
    guard let open = wait(for: { nodes(app).first { node in
        node.role == "AXButton" && ["打开 Markdown", "Open Markdown"].contains(node.text)
    } }) else { throw NSError(domain: "Open Markdown button is unavailable", code: 1) }
    try require(press(open), "Could not press Open Markdown")
    try require(wait(for: { nodes(app).first { ["AXSheet", "AXDialog"].contains($0.role) } }) != nil,
                "The native open dialog did not appear")
    try require(wait(for: { NSWorkspace.shared.frontmostApplication?.processIdentifier == pid ? true : nil }) == true,
                "CodeLab Studio is not the frontmost application")
    postKey(5, flags: [.maskCommand, .maskShift], to: pid) // Go to Folder
    guard let search = wait(for: {
        let tree = nodes(app)
        let sheets = tree.indices.filter { tree[$0].role == "AXSheet" }
        guard sheets.count >= 2, let prompt = sheets.last else { return nil as Node? }
        return tree.dropFirst(prompt).first { $0.role == "AXTextField" }
    }) else { throw NSError(domain: "Could not locate the Go to Folder input", code: 1) }
    let set = AXUIElementSetAttributeValue(search.element, kAXValueAttribute as CFString, path as CFString)
    if set != .success {
        postKey(0, flags: .maskCommand, to: pid)
        postKey(0, text: path, to: pid)
    }
    Thread.sleep(forTimeInterval: 0.3)
    postKey(36, to: pid)
    Thread.sleep(forTimeInterval: 0.35)
    let confirmed = Date()
    postKey(36, to: pid)
    return confirmed
}

let args = CommandLine.arguments
let usage = "Usage: verify-course-preview.swift <pid> <absolute-markdown-path> <title> <output-directory>"
guard args.count == 5, let pid = pid_t(args[1]), AXIsProcessTrusted(),
      let process = NSRunningApplication(processIdentifier: pid),
      process.isTerminated == false else {
    fputs("\(usage)\nA running CodeLab Studio and macOS Accessibility access are required.\n", stderr)
    exit(2)
}
let app = AXUIElementCreateApplication(pid)
AXUIElementSetMessagingTimeout(app, 1)
let output = URL(fileURLWithPath: args[4], isDirectory: true)

do {
    try FileManager.default.createDirectory(at: output, withIntermediateDirectories: true)
    process.activate(options: [])
    try writeSnapshot(nodes(app), to: output.appendingPathComponent("before.ax.tsv"))
    let confirmed = try openMarkdown(args[2], app: app, pid: pid)
    let title = args[3]
    guard let snapshot = wait(for: { () -> [Node]? in
        let snapshot = nodes(app)
        return snapshot.contains { $0.role == "AXHeading" && $0.text == title } ? snapshot : nil
    }) else { throw NSError(domain: "Course preview title did not appear", code: 1) }
    print("Preview visible after \(Int(Date().timeIntervalSince(confirmed) * 1000)) ms")
    try writeSnapshot(snapshot, to: output.appendingPathComponent("preview.ax.tsv"))
    let chapters = snapshot.filter { $0.role == "AXLink" }.map(\.text)
    if title == "Preview fixture" {
        try require(chapters.prefix(2) == ["Exercise 0: Start service", "Exercise 1: Check output"],
                    "The fixture chapter links are missing or out of order")
        try require(snapshot.contains { $0.role == "AXTable" }, "The fixture table is missing")
        try require(snapshot.contains { $0.text == "Course diagram" }, "The image description is missing")
        try require(!snapshot.contains { $0.text.contains("CHECK_SCRIPT_REMOVED") }, "The script content reached the preview")
    } else {
        let exercises = chapters.filter { $0.hasPrefix("练习 ") }
        try require(exercises.count == 10 && exercises.enumerated().allSatisfy { index, title in
            title.hasPrefix("练习 \(index)：")
        }, "Expected the Go/GoFrame exercise links 0 through 9 in order")
    }
    let stepButtons = snapshot.filter { $0.role == "AXDisclosureTriangle" &&
        ["练习三步流", "Exercise steps"].contains($0.text) }
    let stepIndex = title == "Preview fixture" ? 0 : 1
    try require(stepButtons.indices.contains(stepIndex), "The exercise steps are missing")
    try require(press(stepButtons[stepIndex]), "Could not expand the exercise steps")
    let cwd = title == "Preview fixture" ? "web-app" : "task-api"
    guard let expanded = wait(for: { () -> [Node]? in
        let current = nodes(app)
        return current.contains { ["目录 \(cwd)", "Directory \(cwd)"].contains($0.text) }
            && current.contains { ["常驻", "Daemon"].contains($0.text) } ? current : nil
    }) else { throw NSError(domain: "Daemon role or working directory is missing", code: 1) }
    try writeSnapshot(expanded, to: output.appendingPathComponent("steps.ax.tsv"))
    if title == "Preview fixture" {
        guard let debug = expanded.first(where: { $0.role == "AXDisclosureTriangle" &&
            ["调试命令", "Debug commands"].contains($0.text) }) else {
            throw NSError(domain: "The secondary command group is missing", code: 1)
        }
        try require(press(debug), "Could not expand the secondary commands")
        guard let expandedDebug = wait(for: { () -> [Node]? in
            let current = nodes(app)
            return current.contains { ["单次", "One-shot"].contains($0.text) }
                && current.contains { $0.text.contains("checking workspace") } ? current : nil
        }) else { throw NSError(domain: "The one-shot secondary command is missing", code: 1) }
        try writeSnapshot(expandedDebug, to: output.appendingPathComponent("secondary.ax.tsv"))
    }
    if CGPreflightScreenCaptureAccess(), let windows = CGWindowListCopyWindowInfo(.optionOnScreenOnly, kCGNullWindowID) as? [[String: Any]],
       let window = windows.first(where: { $0[kCGWindowOwnerPID as String] as? pid_t == pid &&
           $0[kCGWindowLayer as String] as? Int == 0 }),
       let windowID = window[kCGWindowNumber as String] as? Int {
        let capture = Process()
        capture.executableURL = URL(fileURLWithPath: "/usr/sbin/screencapture")
        capture.arguments = ["-x", "-o", "-l", String(windowID), output.appendingPathComponent("preview.png").path]
        try capture.run()
        capture.waitUntilExit()
        try require(capture.terminationStatus == 0, "Could not capture the preview window")
    }
    guard let close = expanded.first(where: { $0.role == "AXButton" &&
        ["关闭预览", "Close preview"].contains($0.text) }) else {
        throw NSError(domain: "Close preview button is missing", code: 1)
    }
    try require(press(close), "Could not close the preview")
    try require(wait(for: { nodes(app).first { ["打开 Markdown", "Open Markdown"].contains($0.text) } }) != nil,
                "Closing the preview did not return to the desktop shell")
} catch {
    try? writeSnapshot(nodes(app), to: output.appendingPathComponent("failure.ax.tsv"))
    fputs("\(error)\n", stderr)
    exit(1)
}
