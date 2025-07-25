// TODO(#9430): this belongs in re_protos::ext

impl From<re_protos::log_msg::v1alpha1::Compression> for crate::Compression {
    fn from(value: re_protos::log_msg::v1alpha1::Compression) -> Self {
        match value {
            re_protos::log_msg::v1alpha1::Compression::Unspecified
            | re_protos::log_msg::v1alpha1::Compression::None => Self::Off,
            re_protos::log_msg::v1alpha1::Compression::Lz4 => Self::LZ4,
        }
    }
}

impl From<crate::Compression> for re_protos::log_msg::v1alpha1::Compression {
    fn from(value: crate::Compression) -> Self {
        match value {
            crate::Compression::Off => Self::None,
            crate::Compression::LZ4 => Self::Lz4,
        }
    }
}

#[cfg(feature = "decoder")]
#[tracing::instrument(level = "trace", skip_all)]
pub fn log_msg_from_proto(
    message: re_protos::log_msg::v1alpha1::LogMsg,
) -> Result<re_log_types::LogMsg, crate::decoder::DecodeError> {
    re_tracing::profile_function!();

    use re_protos::{log_msg::v1alpha1::log_msg::Msg, missing_field};

    match message.msg {
        Some(Msg::SetStoreInfo(set_store_info)) => Ok(re_log_types::LogMsg::SetStoreInfo(
            set_store_info.try_into()?,
        )),

        Some(Msg::ArrowMsg(arrow_msg)) => {
            let encoded = arrow_msg_from_proto(&arrow_msg)?;

            let store_id: re_log_types::StoreId = arrow_msg
                .store_id
                .ok_or_else(|| missing_field!(re_protos::log_msg::v1alpha1::ArrowMsg, "store_id"))?
                .into();

            Ok(re_log_types::LogMsg::ArrowMsg(store_id, encoded))
        }

        Some(Msg::BlueprintActivationCommand(blueprint_activation_command)) => {
            Ok(re_log_types::LogMsg::BlueprintActivationCommand(
                blueprint_activation_command.try_into()?,
            ))
        }

        None => Err(missing_field!(re_protos::log_msg::v1alpha1::LogMsg, "msg").into()),
    }
}

#[cfg(feature = "decoder")]
#[tracing::instrument(level = "trace", skip_all)]
pub fn arrow_msg_from_proto(
    arrow_msg: &re_protos::log_msg::v1alpha1::ArrowMsg,
) -> Result<re_log_types::ArrowMsg, crate::decoder::DecodeError> {
    use crate::codec::{CodecError, arrow::decode_arrow};
    use crate::decoder::DecodeError;
    use re_protos::log_msg::v1alpha1::Encoding;

    if arrow_msg.encoding() != Encoding::ArrowIpc {
        return Err(DecodeError::Codec(CodecError::UnsupportedEncoding));
    }

    let batch = decode_arrow(
        &arrow_msg.payload,
        arrow_msg.uncompressed_size as usize,
        arrow_msg.compression().into(),
    )?;

    let chunk_id = re_sorbet::chunk_id_of_schema(batch.schema_ref())?.as_tuid();

    Ok(re_log_types::ArrowMsg {
        chunk_id,
        batch,
        on_release: None,
    })
}

#[cfg(feature = "encoder")]
#[tracing::instrument(level = "trace", skip_all)]
pub fn log_msg_to_proto(
    message: re_log_types::LogMsg,
    compression: crate::Compression,
) -> Result<re_protos::log_msg::v1alpha1::LogMsg, crate::encoder::EncodeError> {
    re_tracing::profile_function!();

    use re_protos::log_msg::v1alpha1::{
        BlueprintActivationCommand, LogMsg as ProtoLogMsg, SetStoreInfo,
    };

    let proto_msg = match message {
        re_log_types::LogMsg::SetStoreInfo(set_store_info) => {
            let set_store_info: SetStoreInfo = set_store_info.into();
            ProtoLogMsg {
                msg: Some(re_protos::log_msg::v1alpha1::log_msg::Msg::SetStoreInfo(
                    set_store_info,
                )),
            }
        }

        re_log_types::LogMsg::ArrowMsg(store_id, arrow_msg) => {
            let arrow_msg = arrow_msg_to_proto(&arrow_msg, store_id, compression)?;
            ProtoLogMsg {
                msg: Some(re_protos::log_msg::v1alpha1::log_msg::Msg::ArrowMsg(
                    arrow_msg,
                )),
            }
        }

        re_log_types::LogMsg::BlueprintActivationCommand(blueprint_activation_command) => {
            let blueprint_activation_command: BlueprintActivationCommand =
                blueprint_activation_command.into();
            ProtoLogMsg {
                msg: Some(
                    re_protos::log_msg::v1alpha1::log_msg::Msg::BlueprintActivationCommand(
                        blueprint_activation_command,
                    ),
                ),
            }
        }
    };

    Ok(proto_msg)
}

#[cfg(feature = "encoder")]
#[tracing::instrument(level = "trace", skip_all)]
pub fn arrow_msg_to_proto(
    arrow_msg: &re_log_types::ArrowMsg,
    store_id: re_log_types::StoreId,
    compression: crate::Compression,
) -> Result<re_protos::log_msg::v1alpha1::ArrowMsg, crate::encoder::EncodeError> {
    re_tracing::profile_function!();

    use crate::codec::arrow::encode_arrow;
    use re_protos::log_msg::v1alpha1::ArrowMsg as ProtoArrowMsg;

    let payload = encode_arrow(&arrow_msg.batch, compression)?;

    Ok(ProtoArrowMsg {
        store_id: Some(store_id.into()),
        chunk_id: Some(arrow_msg.chunk_id.into()),
        compression: match compression {
            crate::Compression::Off => re_protos::log_msg::v1alpha1::Compression::None as i32,
            crate::Compression::LZ4 => re_protos::log_msg::v1alpha1::Compression::Lz4 as i32,
        },
        uncompressed_size: payload.uncompressed_size as i32,
        encoding: re_protos::log_msg::v1alpha1::Encoding::ArrowIpc as i32,
        payload: payload.data.into(),
    })
}
