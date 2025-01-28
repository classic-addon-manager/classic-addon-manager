import packageJson from '../package.json';
import {getToken} from "$stores/UserStore.svelte";

const API_URL = 'https://aac.gaijin.dev';
const DEFAULT_HEADERS = {
    'Content-Type': 'application/json',
    'Accept': 'application/json',
    'X-Token': getToken(),
    'X-Client': packageJson.version
}

export const apiClient = {
    get,
    post
}

async function get(url: string): Promise<Response> {
    const response = await fetch(API_URL + url,
        {method: 'GET', headers: DEFAULT_HEADERS}
    );

    if (!response.ok) {
        return Promise.reject(response);
    }

    return response;
}

async function post(url: string, data: object): Promise<Response> {
    const response = await fetch(API_URL + url, {
        method: 'POST',
        body: JSON.stringify(data),
        headers: DEFAULT_HEADERS
    });

    if (!response.ok) {
        return Promise.reject(response);
    }

    return response;
}