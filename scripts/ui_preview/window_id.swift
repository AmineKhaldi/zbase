// Prints the CGWindowID of an on-screen window owned by a process ID: the one
// titled TITLE when given (e.g. "Settings"), otherwise the largest one.
//
//   window_id <pid> [title]
import CoreGraphics
import Foundation

let args = CommandLine.arguments
guard args.count == 2 || args.count == 3, let pid = Int32(args[1]) else {
    FileHandle.standardError.write("usage: window_id <pid> [title]\n".data(using: .utf8)!)
    exit(2)
}
let title: String? = args.count == 3 ? args[2] : nil
let windows = CGWindowListCopyWindowInfo([.optionOnScreenOnly], kCGNullWindowID) as? [[String: Any]] ?? []
let owned = windows.filter { ($0[kCGWindowOwnerPID as String] as? Int32) == pid && ($0[kCGWindowLayer as String] as? Int) == 0 }
func area(_ w: [String: Any]) -> Double {
    let b = w[kCGWindowBounds as String] as? [String: Double] ?? [:]
    return (b["Width"] ?? 0) * (b["Height"] ?? 0)
}
var best = owned.max(by: { area($0) < area($1) })
if let title = title {
    let named = owned.filter { ($0[kCGWindowName as String] as? String) == title }
    if let match = named.max(by: { area($0) < area($1) }) {
        best = match
    } else if owned.count > 1 {
        // Window names need Screen Recording permission. Without it, a titled
        // secondary window (Settings) is the smaller of the app's windows.
        best = owned.min(by: { area($0) < area($1) })
    } else {
        exit(1)
    }
}
guard let window = best, let id = window[kCGWindowNumber as String] as? Int else { exit(1) }
print(id)
