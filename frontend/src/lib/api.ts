import { ApplicationService } from '@/lib/wails'
import { useUserStore } from '@/stores/userStore.ts'

export const API_URL = 'https://aac.gaijin.dev'

function createHeaders(version: string, token: string, os: string): Record<string, string> {
  return {
    'Content-Type': 'application/json',
    Accept: 'application/json',
    'X-Client': version,
    'X-Client-OS': os,
    'X-Token': token,
  }
}

function createFormHeaders(version: string, token: string, os: string): Record<string, string> {
  return {
    Accept: 'application/json',
    'X-Client': version,
    'X-Client-OS': os,
    'X-Token': token,
  }
}

class ApiClient {
  private static instance: ApiClient | null = null
  private version: string | null = null
  private os = 'unknown'
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

    try {
      this.os = await ApplicationService.GetOS()
    } catch (error) {
      console.error('Failed to initialize API client with OS:', error)
    }
  }

  static getInstance(): ApiClient {
    if (!ApiClient.instance) {
      ApiClient.instance = new ApiClient()
    }
    return ApiClient.instance
  }

  private getToken(): string {
    return useUserStore.getState().token || ''
  }

  async get(url: string): Promise<Response> {
    await this.initPromise

    return fetch(API_URL + url, {
      method: 'GET',
      headers: createHeaders(this.version!, this.getToken(), this.os),
    })
  }

  async post(url: string, data?: unknown): Promise<Response> {
    await this.initPromise

    return fetch(API_URL + url, {
      method: 'POST',
      headers: createHeaders(this.version!, this.getToken(), this.os),
      ...(data === undefined ? {} : { body: JSON.stringify(data) }),
    })
  }

  async postForm(url: string, data: FormData): Promise<Response> {
    await this.initPromise

    return fetch(API_URL + url, {
      method: 'POST',
      headers: createFormHeaders(this.version!, this.getToken(), this.os),
      body: data,
    })
  }

  async put(url: string, data: unknown): Promise<Response> {
    await this.initPromise

    return fetch(API_URL + url, {
      method: 'PUT',
      headers: createHeaders(this.version!, this.getToken(), this.os),
      body: JSON.stringify(data),
    })
  }
}

export const apiClient = ApiClient.getInstance()
