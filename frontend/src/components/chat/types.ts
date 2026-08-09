export interface ChatMessageType {
  id: string
  type: 'message'
  role: 'user' | 'assistant'
  content: string
}

/**
 * The stream only tells us that a tool was invoked, never that it finished, so
 * `status` is inferred from whatever event arrives next.
 */
export interface ToolCallMessageType {
  id: string
  type: 'tool_call'
  action: string
  status: 'running' | 'done'
}

export type ChatHistoryItem = ChatMessageType | ToolCallMessageType
