// Notes.swift — note-name and drum-key helpers, the musician-friendly
// entry points for the MCProgrammer library.
//
// The Roland MC family follows the **Roland / Yamaha** octave convention:
// middle C is **C3 = MIDI 60**, so a kick on C1 is MIDI 36. (The
// "scientific pitch notation" used by some software puts middle C at
// C4 = 60, which maps the same kick to C2. We follow Roland here so the
// numbers match what shows on the MC-707 / MC-101 screen.)
//
// Quick reference for the most common drum keys:
//
//   Note.parse("C1")  == 36   // KICK
//   Note.parse("D1")  == 38   // SNARE
//   Note.parse("F#1") == 42   // HAT_CLOSED
//   Note.parse("A#1") == 46   // HAT_OPEN
//
// You can also pass plain MIDI numbers — the parser is a no-op on ints,
// so it's safe to wrap any value the user gives you.

import Foundation

public enum DrumKey {
    public static let KICK         = 36   // C1
    public static let SIDE_STICK   = 37   // C#1
    public static let SNARE        = 38   // D1
    public static let CLAP         = 39   // D#1
    public static let SNARE_RIM    = 40   // E1
    public static let TOM_LOW      = 41   // F1
    public static let HAT_CLOSED   = 42   // F#1
    public static let TOM_LOW_MID  = 43   // G1
    public static let HAT_PEDAL    = 44   // G#1
    public static let TOM_MID      = 45   // A1
    public static let HAT_OPEN     = 46   // A#1
    public static let TOM_HIGH_MID = 47   // B1
    public static let TOM_HIGH     = 48   // C2
    public static let CRASH        = 49   // C#2
    public static let RIDE         = 51   // D#2
}

public enum Note {

    public enum NoteError: Error, CustomStringConvertible {
        case unrecognised(String)
        case outOfRange(Int)

        public var description: String {
            switch self {
            case .unrecognised(let s):
                return "unrecognised note name '\(s)' — expected something " +
                       "like 'C1', 'F#3', 'Bb-1', or a MIDI number 0..127"
            case .outOfRange(let n):
                return "MIDI note out of range: \(n) (must be 0..127)"
            }
        }
    }

    private static let offsets: [String: Int] = [
        "C": 0, "C#": 1, "DB": 1, "D": 2, "D#": 3, "EB": 3, "E": 4,
        "F": 5, "F#": 6, "GB": 6, "G": 7, "G#": 8, "AB": 8,
        "A": 9, "A#": 10, "BB": 10, "B": 11,
    ]

    /// Parse a note name like ``"C1"`` into a MIDI note number 0..127.
    /// Throws ``NoteError`` on invalid input.
    public static func parse(_ name: String) throws -> Int {
        let s = name.trimmingCharacters(in: .whitespaces)
        guard !s.isEmpty else { throw NoteError.unrecognised(name) }

        let upper = s.uppercased()
        let chars = Array(upper)

        // Try the two-character letter first (handles "C#", "Bb", "Eb", …),
        // then fall back to a single-character note. Looking the candidate
        // up directly in the offsets table is the cleanest disambiguator
        // because "B3" and "Bb3" only differ in whether the second char
        // is a digit or a flat — the table only contains valid letters.
        let letter: String
        let restStart: Int
        if chars.count >= 2, offsets[String(chars[0...1])] != nil {
            letter = String(chars[0...1])
            restStart = 2
        } else {
            letter = String(chars[0])
            restStart = 1
        }

        guard let offset = offsets[letter] else {
            throw NoteError.unrecognised(name)
        }

        let octave: Int
        if restStart >= chars.count {
            octave = 3       // Roland convention: C3 is middle C
        } else {
            let rest = String(chars[restStart...])
            guard let parsed = Int(rest) else {
                throw NoteError.unrecognised(name)
            }
            octave = parsed
        }

        let midi = (octave + 2) * 12 + offset
        guard (0...127).contains(midi) else {
            throw NoteError.outOfRange(midi)
        }
        return midi
    }

    /// Accept either a name (``.string("C1")``) or a MIDI number
    /// (``.int(36)``) and return the corresponding 0..127 value.
    public enum Pitch: ExpressibleByIntegerLiteral, ExpressibleByStringLiteral {
        case midi(Int)
        case named(String)

        public init(integerLiteral value: Int)        { self = .midi(value) }
        public init(stringLiteral value: String)      { self = .named(value) }

        public func resolve() throws -> Int {
            switch self {
            case .midi(let n):
                guard (0...127).contains(n) else { throw NoteError.outOfRange(n) }
                return n
            case .named(let s):
                return try Note.parse(s)
            }
        }
    }

    /// Inverse of ``parse(_:)`` — turn a MIDI number back into a name like
    /// ``"C3"``. Returns the sharp form for accidentals (``"C#1"``, never
    /// ``"Db1"``).
    public static func name(of midi: Int) throws -> String {
        guard (0...127).contains(midi) else { throw NoteError.outOfRange(midi) }
        let sharps = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]
        let octave = midi / 12 - 2          // Roland convention (C3 = 60)
        return "\(sharps[midi % 12])\(octave)"
    }
}
