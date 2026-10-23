// Development tooling only: AppKit quits/verifies the exact testing bundle
// without Apple Events, Accessibility, or extra application permissions.
import AppKit

func fail(_ message: String) -> Never {
    FileHandle.standardError.write(Data((message + "\n").utf8))
    exit(1)
}

guard CommandLine.arguments.count == 4 else { fail("Expected mode, app path, and bundle ID.") }
let mode = CommandLine.arguments[1]
let canonical = URL(fileURLWithPath: CommandLine.arguments[2]).standardizedFileURL
let identifier = CommandLine.arguments[3]
func apps() -> [NSRunningApplication] {
    NSWorkspace.shared.runningApplications.filter {
        !$0.isTerminated && ($0.bundleIdentifier == identifier || $0.executableURL?.lastPathComponent == "slotshot")
    }
}
func isCanonical(_ app: NSRunningApplication) -> Bool {
    app.bundleURL?.standardizedFileURL == canonical
}
func rejectOtherBuilds() {
    for app in apps() where !isCanonical(app) {
        fail("Quit the other Slotshot instance first: \(app.executableURL?.path ?? "unknown path"). Only the canonical testing app is stopped automatically.")
    }
}

rejectOtherBuilds()
if mode == "stop" {
    for app in apps() {
        guard app.terminate() else { fail("Slotshot refused to quit. Finish/cancel its current operation and quit it manually.") }
    }
    let deadline = Date().addingTimeInterval(10)
    while !apps().isEmpty && Date() < deadline {
        RunLoop.current.run(until: Date().addingTimeInterval(0.1))
    }
    guard apps().isEmpty else { fail("Slotshot has not quit after 10 seconds; no bundle was replaced. Quit it manually and retry.") }
} else if mode == "verify" {
    let deadline = Date().addingTimeInterval(10)
    while apps().isEmpty && Date() < deadline {
        RunLoop.current.run(until: Date().addingTimeInterval(0.1))
    }
    rejectOtherBuilds()
    guard let app = apps().first, isCanonical(app), app.bundleIdentifier == identifier else {
        fail("The canonical app did not stay running. Check macOS launch/signing errors and quit other Slotshot processes before retrying.")
    }
    print("Verified running PID \(app.processIdentifier): \(app.executableURL!.path)")
} else {
    fail("Unknown mode: \(mode)")
}
