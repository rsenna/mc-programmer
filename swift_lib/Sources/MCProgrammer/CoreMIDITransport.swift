// CoreMIDITransport.swift — a Transport that talks to a Roland MC-101 /
// MC-707 / MV-1 over Apple's CoreMIDI framework.
//
// Usage:
//
//     // Connect to a device whose source and destination endpoints are
//     // both named "MC-707" (the typical Roland default).
//     let transport = try CoreMIDITransport(deviceName: "MC-707")
//     let client = MCClient(transport: transport, modelID: .mc707)
//     let coarse = try client.readCoarseTune(track: 1, clip: nil)
//
// Or pass distinct names if your source and destination endpoints differ:
//
//     let transport = try CoreMIDITransport(
//         sourceName: "MC-707 OUT",
//         destinationName: "MC-707 IN")
//
// The transport is thread-safe and synchronous: `transfer(_:timeout:)`
// blocks the caller while it waits for one Sys-Ex reply. Receive
// framing reassembles the bytes between 0xF0 and 0xF7 boundaries — the
// device may split a long reply across multiple `MIDIPacket`s.

#if canImport(CoreMIDI)
import CoreMIDI
import Foundation

public final class CoreMIDITransport: Transport {

    public enum CoreMIDIError: Error, CustomStringConvertible {
        case clientCreate(OSStatus)
        case inputPortCreate(OSStatus)
        case outputPortCreate(OSStatus)
        case sourceNotFound(String)
        case destinationNotFound(String)
        case connectFailed(OSStatus)
        case sendFailed(OSStatus)

        public var description: String {
            switch self {
            case .clientCreate(let s):     return "MIDIClientCreate failed: \(s)"
            case .inputPortCreate(let s):  return "MIDIInputPortCreate failed: \(s)"
            case .outputPortCreate(let s): return "MIDIOutputPortCreate failed: \(s)"
            case .sourceNotFound(let n):   return "no MIDI source endpoint named \"\(n)\""
            case .destinationNotFound(let n): return "no MIDI destination endpoint named \"\(n)\""
            case .connectFailed(let s):    return "MIDIPortConnectSource failed: \(s)"
            case .sendFailed(let s):       return "MIDISend failed: \(s)"
            }
        }
    }

    private let client: MIDIClientRef
    private let outPort: MIDIPortRef
    private let inPort: MIDIPortRef
    private let source: MIDIEndpointRef
    private let destination: MIDIEndpointRef

    // Receive state. Sys-Ex bytes get appended to `pending` as they arrive.
    // Whenever a 0xF7 is seen we move the assembled message into `frames`
    // and signal `condition` so a blocked `transfer` can wake up.
    private let condition = NSCondition()
    private var pending: [UInt8] = []
    private var frames: [[UInt8]] = []

    // MARK: - Construction

    /// Connect by a single device name (used for both source and destination).
    public convenience init(deviceName: String, clientName: String = "MCProgrammer") throws {
        try self.init(sourceName: deviceName, destinationName: deviceName, clientName: clientName)
    }

    /// Connect by explicit source / destination endpoint names.
    public init(sourceName: String, destinationName: String, clientName: String = "MCProgrammer") throws {
        // 1. Create the MIDI client.
        var client: MIDIClientRef = 0
        var status = MIDIClientCreate(clientName as CFString, nil, nil, &client)
        guard status == noErr else { throw CoreMIDIError.clientCreate(status) }
        self.client = client

        // 2. Find the endpoints.
        guard let source = CoreMIDITransport.findEndpoint(name: sourceName, isSource: true) else {
            throw CoreMIDIError.sourceNotFound(sourceName)
        }
        guard let destination = CoreMIDITransport.findEndpoint(name: destinationName, isSource: false) else {
            throw CoreMIDIError.destinationNotFound(destinationName)
        }
        self.source = source
        self.destination = destination

        // 3. Create the output port.
        var outPort: MIDIPortRef = 0
        status = MIDIOutputPortCreate(client, "\(clientName).out" as CFString, &outPort)
        guard status == noErr else { throw CoreMIDIError.outputPortCreate(status) }
        self.outPort = outPort

        // 4. Create an input port whose receive block defers to `handleIncoming`.
        var inPort: MIDIPortRef = 0
        // We need to capture self by unowned reference inside the C-context
        // block. The block is owned by CoreMIDI for the lifetime of the port.
        var weakSelf: CoreMIDITransport?
        let block: MIDIReadBlock = { packetList, _ in
            weakSelf?.handleIncoming(packetList)
        }
        status = MIDIInputPortCreateWithBlock(
            client, "\(clientName).in" as CFString, &inPort, block)
        guard status == noErr else { throw CoreMIDIError.inputPortCreate(status) }
        self.inPort = inPort
        weakSelf = self  // now safe — we're past the throws

        // 5. Wire the source endpoint to the input port.
        status = MIDIPortConnectSource(inPort, source, nil)
        guard status == noErr else { throw CoreMIDIError.connectFailed(status) }
    }

    deinit {
        MIDIPortDisconnectSource(inPort, source)
        MIDIPortDispose(inPort)
        MIDIPortDispose(outPort)
        MIDIClientDispose(client)
    }

    // MARK: - Endpoint discovery

    private static func findEndpoint(name: String, isSource: Bool) -> MIDIEndpointRef? {
        let count = isSource ? MIDIGetNumberOfSources() : MIDIGetNumberOfDestinations()
        for i in 0..<count {
            let endpoint = isSource ? MIDIGetSource(i) : MIDIGetDestination(i)
            if let ep = endpoint as MIDIEndpointRef?, ep != 0 {
                if let epName = endpointName(ep), epName == name {
                    return ep
                }
            }
        }
        return nil
    }

    private static func endpointName(_ endpoint: MIDIEndpointRef) -> String? {
        var unmanaged: Unmanaged<CFString>?
        let status = MIDIObjectGetStringProperty(endpoint, kMIDIPropertyDisplayName, &unmanaged)
        guard status == noErr, let cfStr = unmanaged?.takeRetainedValue() else { return nil }
        return cfStr as String
    }

    /// Return the display names of all input source endpoints visible to
    /// CoreMIDI. Useful for letting the user pick a device at runtime.
    public static func availableSources() -> [String] {
        return (0..<MIDIGetNumberOfSources()).compactMap {
            endpointName(MIDIGetSource($0))
        }
    }

    public static func availableDestinations() -> [String] {
        return (0..<MIDIGetNumberOfDestinations()).compactMap {
            endpointName(MIDIGetDestination($0))
        }
    }

    // MARK: - Send

    public func sendRaw(_ message: [UInt8]) throws {
        try send(message)
    }

    public func transfer(_ message: [UInt8], timeout: TimeInterval) throws -> [UInt8] {
        // Drain any stale frames before we send — anything older than the
        // request we're about to make is noise.
        condition.lock()
        frames.removeAll()
        pending.removeAll()
        condition.unlock()

        try send(message)

        // Wait for one complete Sys-Ex reply, or for the timeout to expire.
        condition.lock()
        defer { condition.unlock() }
        if frames.isEmpty {
            condition.wait(until: Date().addingTimeInterval(timeout))
        }
        if frames.isEmpty {
            return []
        }
        return frames.removeFirst()
    }

    private func send(_ bytes: [UInt8]) throws {
        // Build a MIDIPacketList containing the message as one packet.
        // For Sys-Ex up to 64KB this is fine; CoreMIDI handles delivery.
        let listSize = MemoryLayout<MIDIPacketList>.size + bytes.count + 32
        let listPtr = UnsafeMutableRawPointer.allocate(
            byteCount: listSize, alignment: MemoryLayout<MIDIPacketList>.alignment)
        defer { listPtr.deallocate() }
        let packetList = listPtr.bindMemory(to: MIDIPacketList.self, capacity: 1)

        let initialPacket = MIDIPacketListInit(packetList)
        bytes.withUnsafeBufferPointer { buf in
            _ = MIDIPacketListAdd(
                packetList,
                listSize,
                initialPacket,
                /* timestamp */ 0,
                bytes.count,
                buf.baseAddress!
            )
        }

        let status = MIDISend(outPort, destination, packetList)
        if status != noErr {
            throw CoreMIDIError.sendFailed(status)
        }
    }

    // MARK: - Receive

    private func handleIncoming(_ list: UnsafePointer<MIDIPacketList>) {
        // Walk the packet list and append every byte. Sys-Ex messages may
        // span packet boundaries, so we accumulate into `pending` and
        // commit a frame each time we see 0xF7.
        condition.lock()
        defer {
            condition.signal()
            condition.unlock()
        }

        var packet = list.pointee.packet
        for _ in 0..<list.pointee.numPackets {
            let length = Int(packet.length)
            withUnsafePointer(to: &packet.data) { dataPtr in
                dataPtr.withMemoryRebound(to: UInt8.self, capacity: length) { bytePtr in
                    for i in 0..<length {
                        let b = bytePtr[i]
                        if pending.isEmpty && b != 0xF0 {
                            // Skip non-Sys-Ex bytes (active sensing, clock, …).
                            continue
                        }
                        pending.append(b)
                        if b == 0xF7 {
                            frames.append(pending)
                            pending.removeAll(keepingCapacity: true)
                        }
                    }
                }
            }
            packet = MIDIPacketNext(&packet).pointee
        }
    }
}

#endif  // canImport(CoreMIDI)
