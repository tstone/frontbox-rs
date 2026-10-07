import type { ClientMessage } from '../types/generated/ClientMessage'
import type { ServerMessage } from '../types/generated/ServerMessage'

export type ConnectionStatus = 'connecting' | 'connected' | 'disconnected'

const MAX_RETRY_DELAY_MS = 5000

export type Connection = {
  send: (message: ClientMessage) => void
  /** Closes the socket for good */
  close: () => void
}

/**
 * Opens the console websocket and keeps it open, reconnecting with backoff when the machine
 * restarts. The server always sends a fresh `Init` on connect, so reconnecting needs no special
 * handling by the caller.
 */
export function connect(
  onMessage: (message: ServerMessage) => void,
  onStatus: (status: ConnectionStatus) => void,
): Connection {
  const url = `${location.protocol === 'https:' ? 'wss' : 'ws'}://${location.host}/ws`
  let socket: WebSocket
  let retries = 0
  let stopped = false

  const open = () => {
    onStatus('connecting')
    socket = new WebSocket(url)
    socket.onopen = () => {
      retries = 0
      onStatus('connected')
    }
    socket.onmessage = (e) => onMessage(JSON.parse(e.data))
    socket.onclose = () => {
      onStatus('disconnected')
      if (!stopped) {
        setTimeout(open, Math.min(MAX_RETRY_DELAY_MS, 250 * 2 ** retries++))
      }
    }
  }

  open()
  return {
    // dropped while disconnected; the server opens anything a client held when it goes away
    send: (message) => {
      if (socket.readyState === WebSocket.OPEN) socket.send(JSON.stringify(message))
    },
    close: () => {
      stopped = true
      socket.close()
    },
  }
}
