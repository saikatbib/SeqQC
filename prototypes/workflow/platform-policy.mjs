export const supportedPlatforms = ['windows', 'macos', 'linux'];

export function checkTaskPlatform(task, platform) {
  if (!supportedPlatforms.includes(platform)) {
    return { allowed: false, code: 'PLATFORM_UNKNOWN', message: `Unsupported platform: ${platform}` };
  }
  if (task.capability === 'alignment' && platform !== 'linux') {
    return {
      allowed: false,
      code: 'ALIGNMENT_REQUIRES_LINUX',
      message: 'Alignment is available only on validated Linux providers in the initial release.'
    };
  }
  if (task.platforms && !task.platforms.includes(platform)) {
    return { allowed: false, code: 'TASK_PLATFORM_UNAVAILABLE', message: `Task is not available on ${platform}.` };
  }
  return { allowed: true, code: 'ALLOWED', message: 'Task is permitted on this platform.' };
}

