// A macOS runner's screen is smaller than the window the scenarios resize, and AppKit keeps a
// window inside its screen: the largest mode the display offers, for the session.
import CoreGraphics

let display = CGMainDisplayID()
let options = [kCGDisplayShowDuplicateLowResolutionModes: kCFBooleanTrue] as CFDictionary
let modes = CGDisplayCopyAllDisplayModes(display, options) as? [CGDisplayMode] ?? []
for mode in modes {
    print("\(mode.width)x\(mode.height), drawn at \(mode.pixelWidth)x\(mode.pixelHeight)")
}

func area(_ mode: CGDisplayMode) -> Int { mode.width * mode.height }
guard let largest = modes.max(by: { area($0) < area($1) }) else {
    fatalError("the display names no mode")
}

// `CGDisplaySetDisplayMode` would last as long as this script.
var configuration: CGDisplayConfigRef?
CGBeginDisplayConfiguration(&configuration)
CGConfigureDisplayWithDisplayMode(configuration, display, largest, nil)
let result = CGCompleteDisplayConfiguration(configuration, .forSession)
print("taken: \(largest.width)x\(largest.height), \(result == .success ? "done" : "refused")")
print("now: \(CGDisplayBounds(display).size)")
