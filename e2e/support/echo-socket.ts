import { createHash } from 'node:crypto';
import { createServer } from 'node:http';
import type { AddressInfo } from 'node:net';
import type { Duplex } from 'node:stream';

/** The constant RFC 6455 appends to the client's key. */
const HANDSHAKE_GUID = '258EAFA5-E914-47DA-95CA-C5AB0DC85B11';

function frame(text: string): Buffer {
  const payload = Buffer.from(text);
  const head =
    payload.length < 126
      ? Buffer.from([0x81, payload.length])
      : Buffer.from([0x81, 126, payload.length >> 8, payload.length & 0xff]);
  return Buffer.concat([head, payload]);
}

/** Reads the client's masked frames off `buffer`; answers each text with `echo: …`. */
function serve(socket: Duplex): void {
  let buffer = Buffer.alloc(0);
  socket.on('data', (chunk: Buffer) => {
    buffer = Buffer.concat([buffer, chunk]);
    while (buffer.length >= 2) {
      const opcode = buffer[0]! & 0x0f;
      let length = buffer[1]! & 0x7f;
      let offset = 2;
      if (length === 126) {
        length = buffer.readUInt16BE(2);
        offset = 4;
      }
      const mask = buffer.subarray(offset, offset + 4);
      offset += 4;
      if (buffer.length < offset + length) return;
      const payload = Buffer.from(buffer.subarray(offset, offset + length)).map(
        (byte, index) => byte ^ mask[index % 4]!,
      );
      buffer = buffer.subarray(offset + length);
      if (opcode === 0x1) socket.write(frame(`echo: ${Buffer.from(payload).toString()}`));
      if (opcode === 0x8) socket.end(Buffer.from([0x88, 0]));
    }
  });
}

/** A WebSocket echo server on the loopback, just enough of RFC 6455 for a scenario. */
export async function echoSocket(): Promise<{ readonly port: number; readonly close: () => void }> {
  const server = createServer();
  const sockets: Duplex[] = [];
  server.on('upgrade', (request, socket) => {
    sockets.push(socket);
    const accept = createHash('sha1')
      .update(`${request.headers['sec-websocket-key'] ?? ''}${HANDSHAKE_GUID}`)
      .digest('base64');
    socket.write(
      [
        'HTTP/1.1 101 Switching Protocols',
        'Upgrade: websocket',
        'Connection: Upgrade',
        `Sec-WebSocket-Accept: ${accept}`,
        '',
        '',
      ].join('\r\n'),
    );
    serve(socket);
  });
  await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve));
  return {
    port: (server.address() as AddressInfo).port,
    close: () => {
      for (const socket of sockets) socket.destroy();
      server.close();
    },
  };
}
