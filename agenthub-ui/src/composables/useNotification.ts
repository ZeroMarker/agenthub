import { ref } from 'vue'

export type MessageType = 'success' | 'error'

export function useNotification() {
  const message = ref('')
  const messageType = ref<MessageType>('success')

  function show(msg: string, type: MessageType = 'success', duration = 5000) {
    message.value = msg
    messageType.value = type
    if (type === 'success' && duration > 0) {
      setTimeout(() => { message.value = '' }, duration)
    }
  }

  function success(msg: string, duration = 5000) {
    show(msg, 'success', duration)
  }

  function error(msg: string) {
    show(msg, 'error', 0) // errors persist
  }

  function clear() {
    message.value = ''
  }

  return { message, messageType, show, success, error, clear }
}
