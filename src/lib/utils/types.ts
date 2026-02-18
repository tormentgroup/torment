export type Message = {
    userIndex: number;
    message: string;
    timestamp: number;
};

export type User = {
    name: string;
    img: string
}

export type RoomStatus = "Joined" | "Left" | "Infited" | "Knocked" | "Banned";

export type RoomInfoMinimal = {
    room_id: string;
    parent_ids: string[];
    status: RoomStatus;
    display_name: string;
    is_space: boolean;
    avatar_url: string;
    children?: RoomInfoMinimal[];
    children_count: number;
};

export type SpaceInfoMinimal = {
    room_id: string;
    display_name: string,
    avatar_url: string,
    children_count: number,
}
