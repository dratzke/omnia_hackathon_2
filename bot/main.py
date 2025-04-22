import logging
import image_util
import marble_client

# Import the generated modules
log_format = '%(asctime)s - %(levelname)s - %(message)s - [%(exception_info)s]'
logging.basicConfig(level=logging.INFO, format=log_format)

log_format = '%(asctime)s - %(levelname)s - %(message)s - [ExceptionType: %(exc_type)s] - [ExceptionMsg: %(exc_msg)s]'


class SafeFormatter(logging.Formatter):
    def format(self, record):
        record.exc_type = getattr(record, 'exc_type', '')
        record.exc_msg = getattr(record, 'exc_msg', '')
        return super().format(record)


# Configure logging using the custom formatter
handler = logging.StreamHandler()
handler.setFormatter(SafeFormatter(log_format))

logger = logging.getLogger()
logger.handlers.clear()
logger.addHandler(handler)
logger.setLevel(logging.INFO)


def run():
    server = image_util.start_server_process(4000, 5000, 1, 30)
    client = image_util.start_client_process(4000, '127.0.0.1', 5001, 'A', 50051)

    bot = marble_client.MarbleClient('localhost', '50051')
    bot.run_interaction_loop(100)
    df = bot.get_records_as_dataframe()
    df.to_csv('marble_client_records.csv', index=False)
    image_util.save_images_from_dataframe(df)

    server.kill()
    client.kill()


if __name__ == '__main__':
    run()
