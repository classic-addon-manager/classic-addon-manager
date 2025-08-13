import { ApplicationService } from '@/lib/wails'
import { useUserStore } from '@/stores/userStore.ts'

const API_URL = 'https://aac.gaijin.dev'

function createHeaders(version: string, token: string): Record<string, string> {
  return {
    'Content-Type': 'application/json',
    Accept: 'application/json',
    'X-Client': version,
    'X-Token': token,
  }
}

class ApiClient {
  private static instance: ApiClient | null = null
  private version: string | null = null
  private readonly initPromise: Promise<void>

  private constructor() {
    this.initPromise = this.initialize()
  }

  private async initialize(): Promise<void> {
    try {
      this.version = await ApplicationService.GetVersion()
    } catch (error) {
      console.error('Failed to initialize API client with version:', error)
      this.version = 'unknown'
    }
  }

  static getInstance(): ApiClient {
    if (!ApiClient.instance) {
      ApiClient.instance = new ApiClient()
    }
    return ApiClient.instance
  }

  async get(url: string): Promise<Response> {
    await this.initPromise

    return fetch(API_URL + url, {
      method: 'GET',
      headers: createHeaders(this.version!, useUserStore.getState().token),
    })
  }

  async post(url: string, data: unknown): Promise<Response> {
    await this.initPromise

    return fetch(API_URL + url, {
      method: 'POST',
      headers: createHeaders(this.version!, useUserStore.getState().token),
      body: JSON.stringify(data),
    })
  }
}

export const apiClient = ApiClient.getInstance()
