export type Message = {
    userIndex: number;
    message: string;
    timestamp: number;
};

export type User = {
    name: string;
    img: string
}

export type RoomInfo = {
    name: string;
    parent: string;
};
