import { app } from 'electron';
import fs from 'fs';
import path from 'path';

import { ILogInput, ILogOutput, LogLevel } from '../shared/logging-types';
import { IpcMainEventChannel } from './ipc-event-channel';

export const OLD_LOG_FILES = ['main.log', 'renderer.log', 'frontend.log'];

export class FileOutput implements ILogOutput {
  private fileDescriptor: number;

  constructor(public level: LogLevel, filePath: string) {
    this.fileDescriptor = fs.openSync(filePath, fs.constants.O_CREAT | fs.constants.O_WRONLY);
  }

  public dispose() {
    fs.closeSync(this.fileDescriptor);
  }

  public write(_level: LogLevel, message: string): Promise<void> {
    return new Promise((resolve, reject) => {
      fs.write(this.fileDescriptor, `${message}\n`, (err) => {
        if (err) {
          reject(err);
        } else {
          resolve();
        }
      });
    });
  }
}

export class IpcInput implements ILogInput {
  public on(handler: (level: LogLevel, message: string) => void) {
    IpcMainEventChannel.logging.handleLog(({ level, message }) => handler(level, message));
  }
}

export function getMainLogPath() {
  return path.join(getLogDirectoryDir(), 'frontend-main.log');
}

export function getRendererLogPath() {
  return path.join(getLogDirectoryDir(), 'frontend-renderer.log');
}

export function createLoggingDirectory(): void {
  fs.mkdirSync(getLogDirectoryDir(), { recursive: true });
}

// When cleaning up old log files they are first backed up and the next time removed.
export function cleanUpLogDirectory(fileNames: string[]): void {
  fileNames.forEach((fileName) => {
    const filePath = path.join(getLogDirectoryDir(), fileName);
    rotateOrDeleteFile(filePath);
  });
}

export function backupLogFile(filePath: string) {
  const backupFilePath = getBackupFilePath(filePath);
  if (fileExists(filePath)) {
    fs.renameSync(filePath, backupFilePath);
  }
}

export function rotateOrDeleteFile(filePath: string): void {
  const backupFilePath = getBackupFilePath(filePath);
  if (fileExists(filePath)) {
    backupLogFile(filePath);
  } else if (fileExists(backupFilePath)) {
    fs.unlinkSync(backupFilePath);
  }
}

function getBackupFilePath(filePath: string): string {
  const parsedPath = path.parse(filePath);
  parsedPath.base = parsedPath.name + '.old' + parsedPath.ext;
  return path.normalize(path.format(parsedPath));
}

function getLogDirectoryDir() {
  return app.getPath('logs');
}

function fileExists(filePath: string): boolean {
  try {
    fs.accessSync(filePath);
    return true;
  } catch (e) {
    return false;
  }
}
