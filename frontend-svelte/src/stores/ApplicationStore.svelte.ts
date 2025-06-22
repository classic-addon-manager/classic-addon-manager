let version: string = $state('');

export function getVersion() {
    return version;
}

export function setVersion(ver: string) {
    version = ver;
}