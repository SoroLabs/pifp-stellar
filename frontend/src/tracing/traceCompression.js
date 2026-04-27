import { gzip, ungzip } from 'pako'

const encoder = new TextEncoder()
const decoder = new TextDecoder()

function uint8ToBase64(bytes) {
    let binary = ''
    const chunkSize = 0x8000

    for (let i = 0; i < bytes.length; i += chunkSize) {
        const chunk = bytes.subarray(i, i + chunkSize)
        binary += String.fromCharCode(...chunk)
    }

    return btoa(binary)
}

function base64ToUint8(base64Value) {
    const binary = atob(base64Value)
    const bytes = new Uint8Array(binary.length)

    for (let i = 0; i < binary.length; i += 1) {
        bytes[i] = binary.charCodeAt(i)
    }

    return bytes
}

export function compressTracePayload(payload, level = 6) {
    const json = JSON.stringify(payload)
    const compressed = gzip(encoder.encode(json), { level })

    return {
        encoding: 'gzip+base64',
        originalBytes: json.length,
        compressedBytes: compressed.length,
        data: uint8ToBase64(compressed),
    }
}

export function decompressTracePayload(packet) {
    if (!packet || packet.encoding !== 'gzip+base64' || typeof packet.data !== 'string') {
        throw new Error('Invalid compressed trace packet')
    }

    const bytes = base64ToUint8(packet.data)
    const json = decoder.decode(ungzip(bytes))
    return JSON.parse(json)
}
