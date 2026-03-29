export const API_BASE_URL = 'http://localhost:3001/api';
export const SOCKET_URL = 'http://localhost:3001';

export async function fetchStories() {
  const response = await fetch(`${API_BASE_URL}/stories`);
  if (!response.ok) throw new Error('Failed to fetch stories');
  return response.json();
}

export async function updateStory(id: string, updates: any) {
  const response = await fetch(`${API_BASE_URL}/stories/${id}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(updates),
  });
  if (!response.ok) throw new Error('Failed to update story');
  return response.json();
}

export async function createStory(story: any) {
  const response = await fetch(`${API_BASE_URL}/stories`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(story),
  });
  if (!response.ok) throw new Error('Failed to create story');
  return response.json();
}
