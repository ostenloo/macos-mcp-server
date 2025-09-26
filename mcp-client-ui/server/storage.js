import { promises as fs } from 'fs';
import path from 'path';

const DATA_DIR = path.resolve(process.cwd(), 'server', 'data');
const CONVERSATIONS_PATH = path.join(DATA_DIR, 'conversations.json');

async function ensureDataFile() {
  await fs.mkdir(DATA_DIR, { recursive: true });
  try {
    await fs.access(CONVERSATIONS_PATH);
  } catch {
    await fs.writeFile(CONVERSATIONS_PATH, '[]', 'utf8');
  }
}

export async function appendConversation(entry) {
  await ensureDataFile();
  const raw = await fs.readFile(CONVERSATIONS_PATH, 'utf8');
  const conversations = JSON.parse(raw);
  conversations.push({ ...entry, timestamp: new Date().toISOString() });
  await fs.writeFile(CONVERSATIONS_PATH, JSON.stringify(conversations, null, 2), 'utf8');
}

export async function listConversations() {
  await ensureDataFile();
  const raw = await fs.readFile(CONVERSATIONS_PATH, 'utf8');
  return JSON.parse(raw);
}
