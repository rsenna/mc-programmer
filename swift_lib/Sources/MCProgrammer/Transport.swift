// Transport.swift — pluggable MIDI transport for the MCProgrammer library.
//
// The library deliberately does *not* depend on CoreMIDI or any specific
// backend — it speaks raw Sys-Ex byte arrays. Provide an implementation of
// `Transport` that knows how to send a Sys-Ex message and collect any
// reply, and the rest of the library will work.

import Foundation

public protocol Transport {
    /// Send a complete Sys-Ex message (including 0xF0/0xF7) and return the
    /// reply bytes, if any. `timeout` is in seconds. An empty array means
    /// *no reply*.
    func transfer(_ message: [UInt8], timeout: TimeInterval) throws -> [UInt8]

    /// Send a non-Sys-Ex MIDI message (Control Change, Program Change, …)
    /// and return immediately. The MC family uses this for Scene and Clip
    /// selection via Program Change — see `MCClient.selectScene(...)`
    /// and `MCClient.selectClip(...)`.
    func sendRaw(_ message: [UInt8]) throws
}

public extension Transport {
    /// Default no-op so existing transports keep building.
    func sendRaw(_ message: [UInt8]) throws {}
}

/// In-memory transport that records every message; useful for tests.
/// Set `responses` to script replies for `transfer`.
public final class NullTransport: Transport {
    public private(set) var sent: [[UInt8]] = []
    public var responses: [[UInt8]] = []

    public init(responses: [[UInt8]] = []) {
        self.responses = responses
    }

    public func transfer(_ message: [UInt8], timeout: TimeInterval) throws -> [UInt8] {
        sent.append(message)
        if responses.isEmpty { return [] }
        return responses.removeFirst()
    }

    public func sendRaw(_ message: [UInt8]) throws {
        sent.append(message)
    }
}
