import {LoggingWinston} from '@google-cloud/logging-winston';
import * as winston from 'winston';

export const logger = winston.createLogger({
  level: 'info',
  format: winston.format.combine(
      winston.format.colorize(),
      winston.format.timestamp(),
      winston.format.align(),
      winston.format.printf((info) => `${info.timestamp} ${info.level} ${info.message}`),
  ),
  transports: [
    new winston.transports.Console({level: 'debug'}),
  ],
});

if (process.env.NODE_ENV === 'production') {
  logger.add(new LoggingWinston());
}
