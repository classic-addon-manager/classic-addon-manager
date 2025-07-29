import {type AddonManifest, LocalAddonService} from '$lib/wails'

import addons from '../addons'

export type DependencyInfo = {
    manifest: AddonManifest;
    isInstalled: boolean;
    depth: number; // Track dependency depth for debugging/logging
};

export type DependencyResolutionResult = {
    dependencies: DependencyInfo[];
    errors: string[];
};

/**
 * Recursively resolves all transitive dependencies for a given addon
 * @param addonManifest The addon to resolve dependencies for
 * @param visited Set of addon names already visited to prevent circular dependencies
 * @param depth Current recursion depth for tracking
 * @returns Promise resolving to all transitive dependencies
 */
export async function resolveTransitiveDependencies(
  addonManifest: AddonManifest,
  visited: Set<string> = new Set(),
  depth: number = 0
): Promise<DependencyResolutionResult> {
  const result: DependencyResolutionResult = {
    dependencies: [],
    errors: []
  }

  // Prevent infinite recursion from circular dependencies
  if (visited.has(addonManifest.name)) {
    result.errors.push(`Circular dependency detected: ${addonManifest.name}`)
    return result
  }

  // Prevent excessive recursion depth
  if (depth > 10) {
    result.errors.push(`Maximum dependency depth exceeded for: ${addonManifest.name}`)
    return result
  }

  visited.add(addonManifest.name)

  if (!addonManifest.dependencies || addonManifest.dependencies.length === 0) {
    visited.delete(addonManifest.name)
    return result
  }

  // Process each direct dependency
  for (const depName of addonManifest.dependencies) {
    try {
      const depManifest = await addons.getManifest(depName)
      const isInstalled = await LocalAddonService.IsInstalled(depName)

      // Add this dependency to our result
      const depInfo: DependencyInfo = {
        manifest: depManifest,
        isInstalled,
        depth
      }

      // Only add if not already in our dependencies list
      if (!result.dependencies.some(d => d.manifest.name === depName)) {
        result.dependencies.push(depInfo)
      }

      // Recursively resolve this dependency's dependencies
      const subResult = await resolveTransitiveDependencies(depManifest, new Set(visited), depth + 1)

      // Merge sub-dependencies, avoiding duplicates
      for (const subDep of subResult.dependencies) {
        if (!result.dependencies.some(d => d.manifest.name === subDep.manifest.name)) {
          result.dependencies.push(subDep)
        }
      }

      result.errors.push(...subResult.errors)

    } catch (error) {
      result.errors.push(`Failed to resolve dependency ${depName}: ${error}`)
      console.error(`Failed to resolve dependency ${depName}:`, error)
    }
  }

  visited.delete(addonManifest.name)
  return result
}

/**
 * Installs dependencies in the correct order (deepest dependencies first)
 * @param dependencies Array of dependencies to install
 * @returns Array of failed dependency names
 */
export async function installDependenciesInOrder(dependencies: DependencyInfo[]): Promise<string[]> {
  // Sort by depth (deepest first) to ensure dependencies are installed before their dependents
  const sortedDeps = [...dependencies].sort((a, b) => b.depth - a.depth)

  const failedDeps: string[] = []

  for (const depInfo of sortedDeps) {
    if (depInfo.isInstalled) {
      console.log(`Dependency ${depInfo.manifest.alias} already installed (depth: ${depInfo.depth}).`)
      continue
    }

    try {
      console.log(`Installing dependency ${depInfo.manifest.alias} (depth: ${depInfo.depth})...`)
      const success = await addons.install(depInfo.manifest, 'latest')
      if (success) {
        depInfo.isInstalled = true
        console.log(`Successfully installed dependency: ${depInfo.manifest.alias}`)
      } else {
        failedDeps.push(depInfo.manifest.name)
        console.error(`Failed to install dependency: ${depInfo.manifest.alias}`)
      }
    } catch (error) {
      failedDeps.push(depInfo.manifest.name)
      console.error(`Error installing dependency ${depInfo.manifest.alias}:`, error)
    }
  }

  return failedDeps
}