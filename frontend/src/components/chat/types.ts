export interface ChatMessageType {
  id: string
  type: 'message'
  role: 'user' | 'assistant'
  content: string
}

export interface ToolCallMessageType {
  id: string
  type: 'tool_call'
  action: string
}

export type ChatHistoryItem = ChatMessageType | ToolCallMessageType
