import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';

// TODO: Replace with actual room fetching logic
function getDefaultRoomId(spaceId: string): string | null {
    const defaultRooms: Record<string, string> = {
        '1': '1',
        '2': '3',
    };
    return defaultRooms[spaceId] ?? null;
}

export const load: PageLoad = ({ params }) => {
    const roomId = getDefaultRoomId(params.spaceId);
    if (roomId) {
        throw redirect(302, `/spaces/${params.spaceId}/room/${roomId}`);
    }
};
