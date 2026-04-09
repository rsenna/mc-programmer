// Protocol.swift — Roland System Exclusive primitives for MC-101, MC-707, MV-1.
//
// The MC family uses the same RQ1 (data request, 0x11) and DT1 (data set, 0x12)
// message format documented for many other Roland instruments. Only the model
// ID byte (index 6) is device-specific. The ZC programming-video transcript
// confirms 0x5D for MC-707 and 0x5E for MC-101.

import Foundation

public enum SysEx {
    public static let start: UInt8 = 0xF0
    public static let end:   UInt8 = 0xF7
    public static let rolandID: UInt8 = 0x41
    public static let deviceID: UInt8 = 0x10

    public static let cmdRQ1: UInt8 = 0x11
    public static let cmdDT1: UInt8 = 0x12
}

public enum ModelID: UInt8 {
    case mc707 = 0x5D
    case mc101 = 0x5E
    case mv1   = 0x5F
}

public enum MCProgrammerError: Error, CustomStringConvertible {
    case messageTooShort(Int)
    case missingFraming
    case notRoland(UInt8)
    case wrongCommand(UInt8)
    case indexOutOfRange(String)
    case valueOutOfRange(String)
    case noDeviceResponse

    public var description: String {
        switch self {
        case .messageTooShort(let n): return "DT1 message too short: \(n) bytes"
        case .missingFraming:         return "Missing Sys-Ex framing bytes"
        case .notRoland(let id):      return String(format: "Not a Roland message (id 0x%02X)", id)
        case .wrongCommand(let cmd):  return String(format: "Unexpected command byte 0x%02X", cmd)
        case .indexOutOfRange(let s): return "Index out of range: \(s)"
        case .valueOutOfRange(let s): return "Value out of range: \(s)"
        case .noDeviceResponse:       return "No response to device inquiry"
        }
    }
}

public enum RolandProtocol {

    /// Standard Roland complement-to-128 checksum over the address + data bytes.
    public static func checksum<S: Sequence>(_ payload: S) -> UInt8 where S.Element == UInt8 {
        let sum = payload.reduce(0) { ($0 + Int($1 & 0x7F)) }
        return UInt8((128 - (sum % 128)) & 0x7F)
    }

    /// Universal device-inquiry message used to detect a Roland model ID.
    public static func universalDeviceInquiry() -> [UInt8] {
        return [0xF0, 0x7E, 0x7F, 0x06, 0x01, 0xF7]
    }

    private static func addressBytes(_ address: UInt32) -> [UInt8] {
        return [
            UInt8((address >> 24) & 0x7F),
            UInt8((address >> 16) & 0x7F),
            UInt8((address >>  8) & 0x7F),
            UInt8( address        & 0x7F),
        ]
    }

    private static func lengthBytes(_ count: UInt32) -> [UInt8] {
        return [
            UInt8((count >> 21) & 0x7F),
            UInt8((count >> 14) & 0x7F),
            UInt8((count >>  7) & 0x7F),
            UInt8( count        & 0x7F),
        ]
    }

    /// Build a complete RQ1 (data request 1) Sys-Ex message.
    public static func buildRQ1(modelID: UInt8, address: UInt32, count: UInt32) -> [UInt8] {
        let header: [UInt8] = [
            SysEx.start, SysEx.rolandID, SysEx.deviceID,
            0x00, 0x00, 0x00, modelID & 0x7F, SysEx.cmdRQ1,
        ]
        let body = addressBytes(address) + lengthBytes(count)
        return header + body + [checksum(body), SysEx.end]
    }

    /// Build a complete DT1 (data set 1) Sys-Ex message.
    public static func buildDT1(modelID: UInt8, address: UInt32, data: [UInt8]) -> [UInt8] {
        let header: [UInt8] = [
            SysEx.start, SysEx.rolandID, SysEx.deviceID,
            0x00, 0x00, 0x00, modelID & 0x7F, SysEx.cmdDT1,
        ]
        let safeData = data.map { $0 & 0x7F }
        let body = addressBytes(address) + safeData
        return header + body + [checksum(body), SysEx.end]
    }

    public struct DT1Payload {
        public let address: UInt32
        public let data: [UInt8]
    }

    /// Extract the address + payload from a complete DT1 reply.
    public static func parseDT1(_ message: [UInt8]) throws -> DT1Payload {
        guard message.count >= 14 else { throw MCProgrammerError.messageTooShort(message.count) }
        guard message.first == SysEx.start, message.last == SysEx.end else {
            throw MCProgrammerError.missingFraming
        }
        guard message[1] == SysEx.rolandID else { throw MCProgrammerError.notRoland(message[1]) }
        guard message[7] == SysEx.cmdDT1   else { throw MCProgrammerError.wrongCommand(message[7]) }

        let address: UInt32 =
              (UInt32(message[8])  << 24)
            | (UInt32(message[9])  << 16)
            | (UInt32(message[10]) <<  8)
            |  UInt32(message[11])
        let data = Array(message[12..<(message.count - 2)])
        return DT1Payload(address: address, data: data)
    }
}
