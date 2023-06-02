import * as path from "https://deno.land/std@0.190.0/path/mod.ts";
import { sample } from "https://deno.land/std@0.188.0/collections/sample.ts";
import "https://deno.land/x/dotenv@v3.2.2/load.ts";
import {
  createBot,
  Intents,
  startBot,
  sendMessage,
  BigString,
} from "https://deno.land/x/discordeno@18.0.1/mod.ts";
import { cron, daily } from "https://deno.land/x/deno_cron@v1.0.0/cron.ts";

export interface WordDefinition {
  word: string;
  phonetic?: string;
  phonetics: Phonetic[];
  meanings: Meaning[];
}

export interface Meaning {
  partOfSpeech: string;
  definitions: Definition[];
  synonyms: string[];
  antonyms: any[];
}

export interface Definition {
  definition: string;
  synonyms: any[];
  antonyms: any[];
}

export interface Phonetic {
  text: string;
  audio: string;
}

const words = Deno.readTextFileSync(
  path.join(Deno.cwd(), "words_alpha.txt")
).split("\n");

const randomWord = (): string => {
  return sample(words) as string;
};

const fetchWord = async (word: string): Promise<WordDefinition | undefined> => {
  const res = await fetch(
    `https://api.dictionaryapi.dev/api/v2/entries/en/${word}`
  );
  if (res.status !== 200) {
    console.warn(`'${word}' could not be defined by dictionary?`);
    return;
  }

  const data = (await res.json()) as WordDefinition[];

  return data.at(0);
};

const randomWordMessage = async (): Promise<string> => {
  const word = await fetchWord(randomWord());
  if (!word) {
    return randomWordMessage();
  }

  if (word.meanings.length === 0) {
    console.error(`what???? MA'AAMM???`);
    return randomWordMessage();
  }
  // ${word.meanings?.[0].definitions?.[0].definition}
  return (
    `${word.word}:\n` +
    word.meanings
      .map(
        (meaning) =>
          `\`${meaning.partOfSpeech}\`` +
          meaning.definitions.map((def) => def.definition).join("\n    ")
      )
      .join("\n  ")
  );
};

const bot = createBot({
  token: Deno.env.get("DISCORD_TOKEN") ?? "",
  intents: Intents.Guilds | Intents.GuildMessages,
  events: {
    ready() {
      console.log("Successfully connected to gateway");
    },
  },
});

const linked_channels: {
  channel_id: BigString;
}[] = [];

// Another way to do events
bot.events.messageCreate = async (b, message) => {
  // ignore bot messages
  if (message.authorId === bot.id) {
    return;
  }

  // check that a user was referenced
  if (message.mentionedUserIds.length !== 1) {
    return;
  }

  // ensure the bot is the only ref
  const [mentioned_user_id] = message.mentionedUserIds;
  if (mentioned_user_id !== bot.id) {
    return;
  }

  // handle edge case that bot is not referenced first
  if (!message.content.startsWith(`<@`)) {
    return;
  }

  // split the commands
  const [cmd, ...subcommands] = message.content.split(" ").slice(1);
  if (!cmd) {
    return;
  }

  switch (cmd) {
    case "help": {
      sendMessage(b, message.channelId, {
        content: `<@${b.id}> \`help\` - This help message.
<@${b.id}> \`new\` - Gives you a new word of the day.
<@${b.id}> \`worddies\` - Registers for a daily word in this channel.`,
      });
      break;
    }
    case "new": {
      const content = await randomWordMessage();
      sendMessage(b, message.channelId, { content });
      break;
    }
    case "worddies": {
      if (linked_channels.includes({ channel_id: message.channelId })) {
        return;
      }

      linked_channels.push({
        channel_id: message.channelId,
      });
      break;
    }
    default:
      console.warn(`unknown command ${cmd}`);
      return;
  }
};

if (import.meta.main) {
  daily(() => {
    linked_channels.forEach(async ({ channel_id }) => {
      const content = await randomWordMessage();
      await sendMessage(bot, channel_id, { content });
    });
  });

  await startBot(bot);
}
