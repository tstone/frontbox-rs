import type { ServerMessage } from '../types/generated/ServerMessage'

export type ConnectionStatus = 'connecting' | 'open' | 'closed'

const MAX_RETRY_DELAY_MS = 5000

/**
 * Opens the console websocket and keeps it open, reconnecting with backoff when the machine
 * restarts. The server always sends a fresh `Init` on connect, so reconnecting needs no special
 * handling by the caller. Returns a function that closes the socket for good.
 */
export function connect(
  onMessage: (message: ServerMessage) => void,
  onStatus: (status: ConnectionStatus) => void,
): () => void {
  const url = `${location.protocol === 'https:' ? 'wss' : 'ws'}://${location.host}/ws`
  let socket: WebSocket
  let retries = 0
  let stopped = false

  const open = () => {
    onStatus('connecting')
    socket = new WebSocket(url)
    socket.onopen = () => {
      retries = 0
      onStatus('open')
    }
    socket.onmessage = (e) => onMessage(JSON.parse(e.data))
    socket.onclose = () => {
      onStatus('closed')
      if (!stopped) {
        setTimeout(open, Math.min(MAX_RETRY_DELAY_MS, 250 * 2 ** retries++))
      }
    }
  }

  open()
  return () => {
    stopped = true
    socket.close()
  }
}
