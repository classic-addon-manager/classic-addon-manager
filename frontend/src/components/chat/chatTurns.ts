import type { ChatHistoryItem, ChatMessageType, ToolCallMessageType } from './types'

export interface UserTurnData {
  kind: 'user'
  id: string
  message: ChatMessageType
}

export interface AssistantTurnData {
  kind: 'assistant'
  id: string
  /** Null when tool calls arrived without an assistant message to attach to. */
  message: ChatMessageType | null
  toolCalls: ToolCallMessageType[]
}

export type ChatTurn = UserTurnData | AssistantTurnData

/**
 * The stream stores tool calls just before the assistant message they belong to,
 * so turns are derived by accumulating tool calls and flushing them onto the
 * next assistant message.
 */
export const buildTurns = (chatHistory: ChatHistoryItem[]): ChatTurn[] => {
  const turns: ChatTurn[] = []
  let pendingToolCalls: ToolCallMessageType[] = []

  for (const item of chatHistory) {
    if (item.type === 'tool_call') {
      pendingToolCalls.push(item)
      continue
    }

    if (item.role === 'user') {
      turns.push({ kind: 'user', id: item.id, message: item })
      continue
    }

    turns.push({
      kind: 'assistant',
      id: item.id,
      message: item,
      toolCalls: pendingToolCalls,
    })
    pendingToolCalls = []
  }

  if (pendingToolCalls.length > 0) {
    turns.push({
      kind: 'assistant',
      id: `tools_${pendingToolCalls[0].id}`,
      message: null,
      toolCalls: pendingToolCalls,
    })
  }

  return turns
}
