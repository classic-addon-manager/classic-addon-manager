import {getToken} from "$stores/UserStore.svelte";
import {getVersion} from "$stores/ApplicationStore.svelte";

const API_URL = 'https://aac.gaijin.dev';
let DEFAULT_HEADERS = {
    'Content-Type': 'application/json',
    'Accept': 'application/json',
    'X-Client': getVersion(),
    'X-Token': ''
}

export const apiClient = {
    get,
    post
}

async function get(url: string): Promise<Response> {
    DEFAULT_HEADERS['X-Token'] = getToken();
    return await fetch(API_URL + url,
        {method: 'GET', headers: DEFAULT_HEADERS}
    );
}

async function post(url: string, data: object): Promise<Response> {
    DEFAULT_HEADERS['X-Token'] = getToken();
    return await fetch(API_URL + url, {
        method: 'POST',
        body: JSON.stringify(data),
        headers: DEFAULT_HEADERS
    });
}