import Foundation
import PDFKit

guard CommandLine.arguments.count == 3 else {
    fputs("Usage: pdf2text <input.pdf> <output.txt>\n", stderr)
    exit(1)
}

let inputURL = URL(fileURLWithPath: CommandLine.arguments[1])
let outputURL = URL(fileURLWithPath: CommandLine.arguments[2])

guard let document = PDFDocument(url: inputURL) else {
    fputs("Failed to open PDF at \(inputURL.path)\n", stderr)
    exit(1)
}

var buffer = ""
let pageCount = document.pageCount
for index in 0..<pageCount {
    autoreleasepool {
        if let page = document.page(at: index), let pageText = page.string {
            buffer.append(pageText)
            if !buffer.hasSuffix("\n") {
                buffer.append("\n")
            }
            buffer.append("\n")
        }
    }
}

do {
    try buffer.write(to: outputURL, atomically: true, encoding: .utf8)
} catch {
    fputs("Failed to write text: \(error)\n", stderr)
    exit(1)
}
