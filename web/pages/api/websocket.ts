import {IncomingMessage} from 'http';
import {MongoClient} from 'mongodb';
import {Duplex} from 'stream';
import {WebSocket, WebSocketServer} from 'ws';

type BoardStatusMessage = {
  deviceId: string; status: string; datetime: string; programId: string;
};

type ZoneAction = {
  deviceId: string; relay_index: number; duration_seconds: number;
}

type Program = {
  id: string; version: number, name: string; weekdays: number[];
  startTime: string;
  zones: ZoneAction[];
}


// Global variable to store WebSocket connections
const subscribers: Set<WebSocket> = new Set<WebSocket>();

// Function to broadcast messages to all connected WebSocket clients
const push_schedule = (message: string) => {
  console.log('Broadcasting message:', message);
  for (const client of subscribers) {
    if (client.readyState === WebSocket.OPEN) {
      client.send(message);
    }
  }
};

// WebSocket server setup
const websocketHandler = (req: any, res: any) => {
  if (res.socket.server.wss) {
    console.log('WebSocket server already running');
    res.end();
    return;
  }

  const wss = new WebSocketServer({noServer: true});

  res.socket.server.on(
      'upgrade',
      (request: IncomingMessage, socket: Duplex,
       head: Buffer<ArrayBufferLike>) => {
        wss.handleUpgrade(request, socket, head, (ws) => {
          wss.emit('connection', ws, request);
        });
      });

  wss.on('connection', (ws) => {
    console.log('New WebSocket connection');
    subscribers.add(ws);

    ws.on('close', () => {
      console.log('WebSocket connection closed');
      subscribers.delete(ws);
    });

    ws.on('message', (message) => {
      try {
        const parsedMessage: BoardStatusMessage =
            JSON.parse(message.toString());
        console.log('Received message:', parsedMessage);

        const uri = `mongodb://${process.env.MONGODB_URI}`;
        const client = new MongoClient(uri);
        const dbName = 'sis';
        const collectionName = 'board_status';

        const upsertBoardStatus = async (message: BoardStatusMessage) => {
          try {
            await client.connect();
            const db = client.db(dbName);
            const collection = db.collection(collectionName);

            await collection.updateOne(
                {deviceId: message.deviceId}, {$set: message}, {upsert: true});

            console.log('Board status upserted successfully');
          } catch (error) {
            console.error('Error upserting board status:', error);
          } finally {
            await client.close();
          }
        };

        upsertBoardStatus(parsedMessage);
      } catch (error) {
        console.error('Invalid message format:', message.toString());
      }
    });
  });

  res.socket.server.wss = wss;
  console.log('WebSocket server started');
  res.end();
};

export {websocketHandler, subscribers};
