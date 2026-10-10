/** Stable hue (0-359) derived from a string, so the same name always gets the same colour. */
export const hueFromString = (value: string): number => {
  let hash = 0
  for (let i = 0; i < value.length; i++) {
    hash = (hash * 31 + value.charCodeAt(i)) % 360
  }
  return hash
}
