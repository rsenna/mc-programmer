// LoopbackTransport.swift — an in-memory fake MC device.
//
// Implements `Transport` by maintaining a sparse in-memory address space:
// every DT1 (data set) message it receives gets written into the store,
// and every RQ1 (data request) it receives is answered with a DT1 reply
// built from the store.
//
// This lets examples and tests run end-to-end without any MIDI hardware,
// using exactly the same `MCClient` calls they would use against a real
// device. No reply staging, no message inspection — just write code that
// looks like the production path.
//
// Universal device-inquiry messages get a synthetic Identity Reply
// containing the configured model ID, so `MCClient.detectModelID(...)`
// also works against a LoopbackTransport.

import Foundation

public final class LoopbackTransport: Transport {

    /// Sparse address-space store: address → byte value.
    private var memory: [UInt32: UInt8] = [:]
    private let modelID: UInt8
    private let unreadByte: UInt8
    private let lock = NSLock()

    /// Every non-Sys-Ex message that's gone through `sendRaw`.
    public private(set) var sentRaw: [[UInt8]] = []
    /// Most-recently-selected scene number (1-based) — updated whenever
    /// any Program Change passes through `sendRaw`.
    public private(set) var activeScene: Int? = nil
    /// Per-MIDI-channel record of the most-recently-selected clip on
    /// that channel (1-based).
    public private(set) var activeClipPerChannel: [Int: Int] = [:]

    /// Create an in-memory fake MC.
    ///
    /// `unreadByte` is what reads from never-written addresses return.
    /// Defaults to `0xFF`, which is what the real MC reports for empty
    /// sequencer steps — note slots are encoded as `0x0F 0x0F` nibble
    /// pairs and decoded as the `MC_EMPTY_NOTE` sentinel `0xFF`. Test
    /// code that wants raw zeroed memory can pass `unreadByte: 0`.
    public init(modelID: ModelID = .mc707, unreadByte: UInt8 = 0xFF) {
        self.modelID = modelID.rawValue
        self.unreadByte = unreadByte
    }

    /// Pre-load a region of the address space (useful in tests when you
    /// want a clip's note step to read back as something specific).
    public func preload(address: UInt32, bytes: [UInt8]) {
        lock.lock(); defer { lock.unlock() }
        for (i, b) in bytes.enumerated() {
            memory[address &+ UInt32(i)] = b
        }
    }

    /// Read back what's currently stored at an address (for assertions).
    /// Returns 0 for never-written addresses regardless of `unreadByte`,
    /// so test code can distinguish "not set" from a deliberately stored
    /// value of `unreadByte`.
    public func snapshot(address: UInt32, count: Int) -> [UInt8] {
        lock.lock(); defer { lock.unlock() }
        return (0..<count).map { memory[address &+ UInt32($0)] ?? 0 }
    }

    // MARK: - Transport

    public func transfer(_ message: [UInt8], timeout: TimeInterval) throws -> [UInt8] {
        // Universal Identity Request → synthetic Identity Reply.
        if isIdentityRequest(message) {
            return identityReply()
        }

        guard message.count >= 14,
              message.first == 0xF0, message.last == 0xF7,
              message[1] == 0x41 else {
            return []
        }

        let cmd = message[7]
        let address: UInt32 =
              (UInt32(message[8])  << 24)
            | (UInt32(message[9])  << 16)
            | (UInt32(message[10]) <<  8)
            |  UInt32(message[11])

        switch cmd {
        case 0x12:   // DT1 — store the payload
            lock.lock()
            for (i, b) in message[12..<(message.count - 2)].enumerated() {
                memory[address &+ UInt32(i)] = b
            }
            lock.unlock()
            return []

        case 0x11:   // RQ1 — synthesise a DT1 reply from the store
            // Length field is the four bytes at indices 12..15 (big-endian
            // 7-bit nibbles, like the address).
            let count: Int =
                  (Int(message[12]) << 21)
                | (Int(message[13]) << 14)
                | (Int(message[14]) <<  7)
                |  Int(message[15])
            lock.lock()
            let data = (0..<count).map { memory[address &+ UInt32($0)] ?? unreadByte }
            lock.unlock()
            return RolandProtocol.buildDT1(modelID: modelID, address: address, data: data)

        default:
            return []
        }
    }

    public func sendRaw(_ message: [UInt8]) throws {
        lock.lock()
        sentRaw.append(message)
        lock.unlock()
        guard !message.isEmpty else { return }
        let status = message[0] & 0xF0
        let channel = Int(message[0] & 0x0F) + 1
        if status == 0xC0 && message.count >= 2 {
            let program = Int(message[1] & 0x7F)
            lock.lock()
            activeScene = program + 1
            activeClipPerChannel[channel] = program + 1
            lock.unlock()
        }
    }

    // MARK: - Helpers

    private func isIdentityRequest(_ msg: [UInt8]) -> Bool {
        // F0 7E <dev> 06 01 F7
        return msg.count == 6
            && msg[0] == 0xF0
            && msg[1] == 0x7E
            && msg[3] == 0x06
            && msg[4] == 0x01
            && msg[5] == 0xF7
    }

    private func identityReply() -> [UInt8] {
        // The MCClient detectModelID just reads byte index 6 of the reply.
        // Build a minimal Identity Reply that places the model ID there:
        //   F0 7E 7F 06 02 41 <modelID> ... F7
        return [0xF0, 0x7E, 0x7F, 0x06, 0x02, 0x41,
                modelID, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF7]
    }
}
