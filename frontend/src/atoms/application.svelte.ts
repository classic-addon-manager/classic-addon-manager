let version: string = $state('')

export function getVersion(): string {
  return version
}

export function setVersion(ver: string) {
  version = ver
}