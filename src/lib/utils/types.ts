export type Message = {
    userIndex: number;
    message: string;
    timestamp: number;
};

export type User = {
    name: string;
    img: string
}

export type SpaceInfo = {
    name: string;
    id: string;
    img: string;
};

// TODO: Remove/Replace with the big definition
export type RoomInfo = {
    display_name: string;
    id: string;
    kind: string;
};
