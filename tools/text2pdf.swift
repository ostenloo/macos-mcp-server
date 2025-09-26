import Foundation
import CoreGraphics
import CoreText

guard CommandLine.arguments.count == 3 else {
    fputs("Usage: text2pdf <input.txt> <output.pdf>\n", stderr)
    exit(1)
}

let inputPath = CommandLine.arguments[1]
let outputPath = CommandLine.arguments[2]
let inputURL = URL(fileURLWithPath: inputPath)
let outputURL = URL(fileURLWithPath: outputPath)

let fileData: Data

do {
    fileData = try Data(contentsOf: inputURL)
} catch {
    fputs("Failed to read input file: \(error)\n", stderr)
    exit(1)
}

let stringEncodings: [String.Encoding] = [.utf8, .utf16LittleEndian, .utf16BigEndian, .macOSRoman]
var decoded: String?
for encoding in stringEncodings {
    decoded = String(data: fileData, encoding: encoding)
    if decoded != nil {
        break
    }
}
let originalText = decoded ?? String(decoding: fileData, as: UTF8.self)

let normalizedText = originalText
    .replacingOccurrences(of: "\r\n", with: "\n")
    .replacingOccurrences(of: "\r", with: "\n")
    .replacingOccurrences(of: "\t", with: "    ")

let attributed = NSMutableAttributedString(string: normalizedText)
let fullRange = NSRange(location: 0, length: attributed.length)

let fontName = "Menlo" as CFString
let fontSize: CGFloat = 10.0
let font = CTFontCreateWithName(fontName, fontSize, nil)

attributed.addAttribute(NSAttributedString.Key(kCTFontAttributeName as String), value: font, range: fullRange)

var zero: CGFloat = 0.0
var lineSpacing: CGFloat = 2.0
var paragraphSettings: [CTParagraphStyleSetting] = [
    CTParagraphStyleSetting(spec: .lineSpacingAdjustment, valueSize: MemoryLayout.size(ofValue: lineSpacing), value: &lineSpacing),
    CTParagraphStyleSetting(spec: .paragraphSpacingBefore, valueSize: MemoryLayout.size(ofValue: zero), value: &zero),
    CTParagraphStyleSetting(spec: .paragraphSpacing, valueSize: MemoryLayout.size(ofValue: zero), value: &zero)
]
let paragraphStyle = CTParagraphStyleCreate(&paragraphSettings, paragraphSettings.count)
attributed.addAttribute(NSAttributedString.Key(kCTParagraphStyleAttributeName as String), value: paragraphStyle, range: fullRange)

let framesetter = CTFramesetterCreateWithAttributedString(attributed)

let pageRect = CGRect(x: 0, y: 0, width: 612, height: 792)
let margin: CGFloat = 36
let textRect = pageRect.insetBy(dx: margin, dy: margin)

var mediaBox = pageRect

guard let consumer = CGDataConsumer(url: outputURL as CFURL),
      let context = CGContext(consumer: consumer, mediaBox: &mediaBox, nil) else {
    fputs("Failed to create PDF context.\n", stderr)
    exit(1)
}

var currentRange = CFRange(location: 0, length: 0)
let path = CGMutablePath()
path.addRect(textRect)

repeat {
    context.beginPDFPage(nil)
    context.saveGState()
    context.translateBy(x: 0, y: pageRect.height)
    context.scaleBy(x: 1, y: -1)

    let frame = CTFramesetterCreateFrame(framesetter, currentRange, path, nil)
    CTFrameDraw(frame, context)

    let visibleRange = CTFrameGetVisibleStringRange(frame)
    if visibleRange.length == 0 {
        context.restoreGState()
        context.endPDFPage()
        break
    }

    currentRange = CFRange(location: visibleRange.location + visibleRange.length, length: 0)

    context.restoreGState()
    context.endPDFPage()
} while currentRange.location < attributed.length

context.closePDF()
