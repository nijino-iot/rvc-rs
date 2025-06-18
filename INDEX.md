# Project Python Index

## Third-party Libraries

- FreeSimpleGUI
- av
- cv2
- dotenv
- fairseq
- faiss
- fastapi
- ffmpeg
- gradio
- intel_extension_for_pytorch
- librosa
- matplotlib
- numpy
- onnxruntime
- onnxsim
- parselmouth
- pydantic
- pyworld
- requests
- scipy
- sklearn
- sounddevice
- soundfile
- torch
- torch_directml
- torchaudio
- torchcrepe
- torchfcpe
- tqdm
- uvicorn

## Modules

### api_231006.py
* Functions: get_input_devices (L358-364), get_output_devices (L367-373), configure_audio (L376-392), start_conversion (L395-408), stop_conversion (L411-426), __main__ (L428-440)
* Classes:
  - GUIConfig (L27-44): __init__ (L28-44)
  - ConfigData (L46-60)
  - AudioAPI (L62-353): __init__ (L63-69), load (L71-101), set_values (L103-124), start_vc (L126-204), soundinput (L206-220), audio_callback (L222-292), get_devices (L294-328), set_devices (L330-353)

### api_240604.py
* Functions: get_input_devices (L484-490), get_output_devices (L493-499), configure_audio (L502-517), start_conversion (L520-533), stop_conversion (L536-551), __main__ (L553-565)
* Classes:
  - GUIConfig (L28-47): __init__ (L29-47)
  - ConfigData (L49-66)
  - Harvest (L68-88): __init__ (L69-72), run (L74-88)
  - AudioAPI (L90-479): __init__ (L91-100), initialize_queues (L102-108), load (L110-141), set_values (L143-166), start_vc (L168-277), soundinput (L279-293), audio_callback (L295-418), get_devices (L420-454), set_devices (L456-479)

### configs/config.py
* Functions: singleton_variable (L33-40), singleton_variable.wrapper (L34-37)
* Classes:
  - Config (L44-254): __init__ (L45-63), load_config_json (L66-74), arg_parse (L77-107), has_mps (L112-119), has_xpu (L122-126), use_fp32_config (L128-137), device_config (L139-254)

### generate_index.py
* Functions: is_third_party (L26-33)
* Classes:
  - IndexVisitor (L38-89): __init__ (L39-43), visit_FunctionDef (L45-53), visit_ClassDef (L55-70), visit_If (L72-89)

### gui_v1.py
* Functions: printt (L19-23), phase_vocoder (L26-47), __main__ (L74-1070)
* Classes:
  - Harvest (L50-71): __init__ (L51-54), run (L56-71)
  - GUIConfig (L114-135): __init__ (L115-135)
  - GUI (L137-1068): __init__ (L138-150), load (L152-221), launcher (L223-532), event_handler (L534-653), set_values (L655-705), start_vc (L707-816), start_stream (L818-837), stop_stream (L839-846), audio_callback (L848-1008), update_devices (L1010-1043), set_devices (L1045-1054), get_device_samplerate (L1056-1059), get_device_channels (L1061-1068)

### i18n/i18n.py
* Functions: load_language_list (L6-9)
* Classes:
  - I18nAuto (L12-27): __init__ (L13-21), __call__ (L23-24), __repr__ (L26-27)

### i18n/locale_diff.py

### i18n/scan_i18n.py
* Functions: extract_i18n_strings (L7-22)

### infer/lib/audio.py
* Functions: wav2 (L10-30), load_audio (L33-52), clean_path (L56-60)

### infer/lib/infer_pack/attentions.py
* Classes:
  - Encoder (L14-77): __init__ (L15-60), forward (L62-77)
  - Decoder (L80-163): __init__ (L81-138), forward (L140-163)
  - MultiHeadAttention (L166-385): __init__ (L167-218), forward (L220-230), attention (L232-288), _matmul_with_relative_values (L290-297), _matmul_with_relative_keys (L299-306), _get_relative_embeddings (L308-325), _relative_position_to_absolute_position (L327-352), _absolute_position_to_relative_position (L354-374), _attention_bias_proximal (L376-385)
  - FFN (L388-459): __init__ (L389-415), padding (L417-422), forward (L424-433), _causal_padding (L435-446), _same_padding (L448-459)

### infer/lib/infer_pack/attentions_onnx.py
* Classes:
  - Encoder (L22-85): __init__ (L23-68), forward (L70-85)
  - Decoder (L88-171): __init__ (L89-146), forward (L148-171)
  - MultiHeadAttention (L174-385): __init__ (L175-226), forward (L228-238), attention (L240-293), _matmul_with_relative_values (L295-302), _matmul_with_relative_keys (L304-311), _get_relative_embeddings (L313-328), _relative_position_to_absolute_position (L330-354), _absolute_position_to_relative_position (L356-374), _attention_bias_proximal (L376-385)
  - FFN (L388-459): __init__ (L389-415), padding (L417-422), forward (L424-433), _causal_padding (L435-446), _same_padding (L448-459)

### infer/lib/infer_pack/commons.py
* Functions: init_weights (L10-13), get_padding (L16-17), kl_divergence (L26-32), rand_gumbel (L35-38), rand_gumbel_like (L41-43), slice_segments (L46-52), slice_segments2 (L55-61), rand_slice_segments (L64-71), get_timing_signal_1d (L74-87), add_timing_signal_1d (L90-93), cat_timing_signal_1d (L96-99), subsequent_mask (L102-104), fused_add_tanh_sigmoid_multiply (L108-114), convert_pad_shape (L123-124), shift_1d (L127-129), sequence_mask (L132-136), generate_path (L139-154), clip_grad_value_ (L157-172)

### infer/lib/infer_pack/models.py
* Classes:
  - TextEncoder (L19-79): __init__ (L20-52), forward (L54-79)
  - ResidualCouplingBlock (L82-145): __init__ (L83-115), forward (L117-130), remove_weight_norm (L132-134), __prepare_scriptable__ (L136-145)
  - PosteriorEncoder (L148-201): __init__ (L149-176), forward (L178-189), remove_weight_norm (L191-192), __prepare_scriptable__ (L194-201)
  - Generator (L204-309): __init__ (L205-250), forward (L252-281), __prepare_scriptable__ (L283-303), remove_weight_norm (L305-309)
  - SineGen (L312-388): __init__ (L328-343), _f02uv (L345-351), _f02sine (L353-369), forward (L371-388)
  - SourceModuleHnNSF (L391-445): __init__ (L409-430), forward (L433-445)
  - GeneratorNSF (L448-592): __init__ (L449-520), forward (L522-565), remove_weight_norm (L567-571), __prepare_scriptable__ (L573-592)
  - SynthesizerTrnMs256NSFsid (L602-776): __init__ (L603-686), remove_weight_norm (L688-692), __prepare_scriptable__ (L694-718), forward (L721-743), infer (L746-776)
  - SynthesizerTrnMs768NSFsid (L779-833): __init__ (L780-833)
  - SynthesizerTrnMs256NSFsid_nono (L836-991): __init__ (L837-917), remove_weight_norm (L919-923), __prepare_scriptable__ (L925-949), forward (L952-961), infer (L964-991)
  - SynthesizerTrnMs768NSFsid_nono (L994-1049): __init__ (L995-1049)
  - MultiPeriodDiscriminator (L1052-1079): __init__ (L1053-1062), forward (L1064-1079)
  - MultiPeriodDiscriminatorV2 (L1082-1109): __init__ (L1083-1092), forward (L1094-1109)
  - DiscriminatorS (L1112-1139): __init__ (L1113-1126), forward (L1128-1139)
  - DiscriminatorP (L1142-1223): __init__ (L1143-1197), forward (L1199-1223)

### infer/lib/infer_pack/models_onnx.py
* Classes:
  - TextEncoder256 (L27-71): __init__ (L28-54), forward (L56-71)
  - TextEncoder768 (L74-118): __init__ (L75-101), forward (L103-118)
  - ResidualCouplingBlock (L121-167): __init__ (L122-154), forward (L156-163), remove_weight_norm (L165-167)
  - PosteriorEncoder (L170-212): __init__ (L171-198), forward (L200-209), remove_weight_norm (L211-212)
  - Generator (L215-288): __init__ (L216-261), forward (L263-282), remove_weight_norm (L284-288)
  - SineGen (L291-367): __init__ (L307-322), _f02uv (L324-330), _f02sine (L332-348), forward (L350-367)
  - SourceModuleHnNSF (L370-416): __init__ (L388-409), forward (L411-416)
  - GeneratorNSF (L419-519): __init__ (L420-489), forward (L491-513), remove_weight_norm (L515-519)
  - SynthesizerTrnMsNSFsidM (L529-649): __init__ (L530-622), remove_weight_norm (L624-627), construct_spkmixmap (L629-633), forward (L635-649)
  - MultiPeriodDiscriminator (L652-679): __init__ (L653-662), forward (L664-679)
  - MultiPeriodDiscriminatorV2 (L682-709): __init__ (L683-692), forward (L694-709)
  - DiscriminatorS (L712-739): __init__ (L713-726), forward (L728-739)
  - DiscriminatorP (L742-818): __init__ (L743-797), forward (L799-818)

### infer/lib/infer_pack/modules/F0Predictor/DioF0Predictor.py
* Classes:
  - DioF0Predictor (L7-91): __init__ (L8-12), interpolate_f0 (L14-50), resize_f0 (L52-61), compute_f0 (L63-76), compute_f0_uv (L78-91)

### infer/lib/infer_pack/modules/F0Predictor/F0Predictor.py
* Classes:
  - F0Predictor (L1-16): compute_f0 (L2-8), compute_f0_uv (L10-16)

### infer/lib/infer_pack/modules/F0Predictor/HarvestF0Predictor.py
* Classes:
  - HarvestF0Predictor (L7-87): __init__ (L8-12), interpolate_f0 (L14-50), resize_f0 (L52-61), compute_f0 (L63-74), compute_f0_uv (L76-87)

### infer/lib/infer_pack/modules/F0Predictor/PMF0Predictor.py
* Classes:
  - PMF0Predictor (L7-98): __init__ (L8-12), interpolate_f0 (L14-50), compute_f0 (L52-74), compute_f0_uv (L76-98)

### infer/lib/infer_pack/modules/F0Predictor/__init__.py

### infer/lib/infer_pack/modules.py
* Constants: LRELU_SLOPE
* Classes:
  - LayerNorm (L20-32): __init__ (L21-27), forward (L29-32)
  - ConvReluNorm (L35-84): __init__ (L36-75), forward (L77-84)
  - DDSConv (L87-133): __init__ (L92-119), forward (L121-133)
  - WN (L136-249): __init__ (L137-186), forward (L188-217), remove_weight_norm (L219-225), __prepare_scriptable__ (L227-249)
  - ResBlock1 (L252-364): __init__ (L253-326), forward (L328-341), remove_weight_norm (L343-347), __prepare_scriptable__ (L349-364)
  - ResBlock2 (L367-420): __init__ (L368-395), forward (L397-406), remove_weight_norm (L408-410), __prepare_scriptable__ (L412-420)
  - Log (L423-437): forward (L424-437)
  - Flip (L440-456): forward (L444-456)
  - ElementwiseAffine (L459-474): __init__ (L460-464), forward (L466-474)
  - ResidualCouplingLayer (L477-549): __init__ (L478-510), forward (L512-537), remove_weight_norm (L539-540), __prepare_scriptable__ (L542-549)
  - ConvFlow (L552-615): __init__ (L553-577), forward (L579-615)

### infer/lib/infer_pack/onnx_inference.py
* Functions: get_f0_predictor (L38-61)
* Classes:
  - ContentVec (L11-35): __init__ (L12-22), __call__ (L24-25), forward (L27-35)
  - OnnxRVC (L64-149): __init__ (L65-85), forward (L87-96), inference (L98-149)

### infer/lib/infer_pack/transforms.py
* Constants: DEFAULT_MIN_BIN_WIDTH, DEFAULT_MIN_BIN_HEIGHT, DEFAULT_MIN_DERIVATIVE
* Functions: piecewise_rational_quadratic_transform (L10-40), searchsorted (L43-45), unconstrained_rational_quadratic_spline (L48-95), rational_quadratic_spline (L98-207)

### infer/lib/jit/__init__.py
* Functions: load_inputs (L9-17), benchmark (L20-30), jit_warm_up (L33-34), to_jit_model (L37-73), export (L76-99), load (L102-104), save (L107-109), rmvpe_jit_export (L112-134), synthesizer_jit_export (L137-163)

### infer/lib/jit/get_hubert.py
* Functions: pad_to_multiple (L14-25), extract_features (L28-92), extract_features.undo_pad (L83-88), compute_mask_indices (L95-224), compute_mask_indices.arrange (L167-176), apply_mask (L227-263), get_hubert_model (L266-342), get_hubert_model._apply_mask (L276-277), get_hubert_model._extract_features (L281-293), get_hubert_model.hubert_extract_features (L299-315), get_hubert_model._hubert_extract_features (L317-326), get_hubert_model.infer (L330-336)

### infer/lib/jit/get_rmvpe.py
* Functions: get_rmvpe (L4-12)

### infer/lib/jit/get_synthesizer.py
* Functions: get_synthesizer (L4-38)

### infer/lib/rmvpe.py
* Functions: __main__ (L649-670)
* Classes:
  - STFT (L29-156): __init__ (L30-76), transform (L78-107), inverse (L109-142), forward (L144-156)
  - BiGRU (L162-174): __init__ (L163-171), forward (L173-174)
  - ConvBlockRes (L177-210): __init__ (L178-204), forward (L206-210)
  - Encoder (L213-248): __init__ (L214-240), forward (L242-248)
  - ResEncoderBlock (L251-271): __init__ (L252-263), forward (L265-271)
  - Intermediate (L274-290): __init__ (L275-285), forward (L287-290)
  - ResDecoderBlock (L293-321): __init__ (L294-314), forward (L316-321)
  - Decoder (L324-339): __init__ (L325-334), forward (L336-339)
  - DeepUnet (L342-370): __init__ (L343-364), forward (L366-370)
  - E2E (L373-412): __init__ (L374-404), forward (L406-412)
  - MelSpectrogram (L418-492): __init__ (L419-450), forward (L452-492)
  - RMVPE (L495-646): __init__ (L496-567), mel2hidden (L569-585), decode (L587-592), infer_from_audio (L594-620), to_local_average_cents (L622-646)

### infer/lib/rtrvc.py
* Functions: printt (L31-35)
* Classes:
  - RVC (L40-461): __init__ (L41-190), change_key (L192-193), change_formant (L195-196), change_index_rate (L198-203), get_f0_post (L205-216), get_f0 (L218-287), get_f0_crepe (L289-311), get_f0_rmvpe (L313-326), get_f0_fcpe (L328-345), infer (L347-461)

### infer/lib/slicer2.py
* Functions: get_rms (L5-35), main (L182-256), __main__ (L259-260)
* Classes:
  - Slicer (L38-179): __init__ (L39-62), _apply_slice (L64-72), slice (L75-179)

### infer/lib/train/data_utils.py
* Classes:
  - TextAudioLoaderMultiNSFsid (L15-144): __init__ (L22-32), _filter (L34-48), get_sid (L50-52), get_audio_text_pair (L54-81), get_labels (L83-96), get_audio (L98-138), __getitem__ (L140-141), __len__ (L143-144)
  - TextAudioCollateMultiNSFsid (L147-220): __init__ (L150-151), __call__ (L153-220)
  - TextAudioLoader (L223-336): __init__ (L230-240), _filter (L242-256), get_sid (L258-260), get_audio_text_pair (L262-280), get_labels (L282-288), get_audio (L290-330), __getitem__ (L332-333), __len__ (L335-336)
  - TextAudioCollate (L339-398): __init__ (L342-343), __call__ (L345-398)
  - DistributedBucketSampler (L401-517): __init__ (L411-427), _create_buckets (L429-450), __iter__ (L452-499), _bisect (L501-514), __len__ (L516-517)

### infer/lib/train/losses.py
* Functions: feature_loss (L4-12), discriminator_loss (L15-28), generator_loss (L31-40), kl_loss (L43-58)

### infer/lib/train/mel_processing.py
* Constants: MAX_WAV_VALUE
* Functions: dynamic_range_compression_torch (L11-17), dynamic_range_decompression_torch (L20-26), spectral_normalize_torch (L29-30), spectral_de_normalize_torch (L33-34), spectrogram_torch (L42-89), spec_to_mel_torch (L92-108), mel_spectrogram_torch (L111-127)

### infer/lib/train/process_ckpt.py
* Functions: savee (L13-48), show_info (L51-61), extract_small_model (L64-191), change_info (L194-203), merge (L206-261), merge.extract (L209-217)

### infer/lib/train/utils.py
* Constants: MATPLOTLIB_FLAG
* Functions: load_checkpoint_d (L20-68), load_checkpoint_d.go (L25-51), load_checkpoint (L100-141), save_checkpoint (L144-162), save_checkpoint_d (L165-188), summarize (L191-207), latest_checkpoint_path (L210-215), plot_spectrogram_to_numpy (L218-241), plot_alignment_to_numpy (L244-272), load_wav_to_torch (L275-277), load_filepaths_and_text (L280-288), get_hparams (L291-391), get_hparams_from_dir (L394-402), get_hparams_from_file (L405-411), check_git_hash (L414-436), get_logger (L439-451)
* Classes:
  - HParams (L454-483): __init__ (L455-459), keys (L461-462), items (L464-465), values (L467-468), __len__ (L470-471), __getitem__ (L473-474), __setitem__ (L476-477), __contains__ (L479-480), __repr__ (L482-483)

### infer/lib/uvr5_pack/lib_v5/dataset.py
* Functions: make_pair (L31-51), train_val_split (L54-87), augment (L90-115), make_padding (L118-125), make_training_set (L128-150), make_validation_set (L153-183)
* Classes:
  - VocalRemoverValidationSet (L12-28): __init__ (L13-14), __len__ (L16-17), __getitem__ (L19-28)

### infer/lib/uvr5_pack/lib_v5/layers.py
* Classes:
  - Conv2DBNActiv (L8-26): __init__ (L9-23), __call__ (L25-26)
  - SeperableConv2DBNActiv (L29-49): __init__ (L30-46), __call__ (L48-49)
  - Encoder (L52-62): __init__ (L53-56), __call__ (L58-62)
  - Decoder (L65-83): __init__ (L66-71), __call__ (L73-83)
  - ASPPModule (L86-118): __init__ (L87-105), forward (L107-118)

### infer/lib/uvr5_pack/lib_v5/layers_123812KB .py
* Classes:
  - Conv2DBNActiv (L8-26): __init__ (L9-23), __call__ (L25-26)
  - SeperableConv2DBNActiv (L29-49): __init__ (L30-46), __call__ (L48-49)
  - Encoder (L52-62): __init__ (L53-56), __call__ (L58-62)
  - Decoder (L65-83): __init__ (L66-71), __call__ (L73-83)
  - ASPPModule (L86-118): __init__ (L87-105), forward (L107-118)

### infer/lib/uvr5_pack/lib_v5/layers_123821KB.py
* Classes:
  - Conv2DBNActiv (L8-26): __init__ (L9-23), __call__ (L25-26)
  - SeperableConv2DBNActiv (L29-49): __init__ (L30-46), __call__ (L48-49)
  - Encoder (L52-62): __init__ (L53-56), __call__ (L58-62)
  - Decoder (L65-83): __init__ (L66-71), __call__ (L73-83)
  - ASPPModule (L86-118): __init__ (L87-105), forward (L107-118)

### infer/lib/uvr5_pack/lib_v5/layers_33966KB.py
* Classes:
  - Conv2DBNActiv (L8-26): __init__ (L9-23), __call__ (L25-26)
  - SeperableConv2DBNActiv (L29-49): __init__ (L30-46), __call__ (L48-49)
  - Encoder (L52-62): __init__ (L53-56), __call__ (L58-62)
  - Decoder (L65-83): __init__ (L66-71), __call__ (L73-83)
  - ASPPModule (L86-126): __init__ (L87-111), forward (L113-126)

### infer/lib/uvr5_pack/lib_v5/layers_537227KB.py
* Classes:
  - Conv2DBNActiv (L8-26): __init__ (L9-23), __call__ (L25-26)
  - SeperableConv2DBNActiv (L29-49): __init__ (L30-46), __call__ (L48-49)
  - Encoder (L52-62): __init__ (L53-56), __call__ (L58-62)
  - Decoder (L65-83): __init__ (L66-71), __call__ (L73-83)
  - ASPPModule (L86-126): __init__ (L87-111), forward (L113-126)

### infer/lib/uvr5_pack/lib_v5/layers_537238KB.py
* Classes:
  - Conv2DBNActiv (L8-26): __init__ (L9-23), __call__ (L25-26)
  - SeperableConv2DBNActiv (L29-49): __init__ (L30-46), __call__ (L48-49)
  - Encoder (L52-62): __init__ (L53-56), __call__ (L58-62)
  - Decoder (L65-83): __init__ (L66-71), __call__ (L73-83)
  - ASPPModule (L86-126): __init__ (L87-111), forward (L113-126)

### infer/lib/uvr5_pack/lib_v5/layers_new.py
* Classes:
  - Conv2DBNActiv (L8-26): __init__ (L9-23), __call__ (L25-26)
  - Encoder (L29-39): __init__ (L30-33), __call__ (L35-39)
  - Decoder (L42-64): __init__ (L43-49), __call__ (L51-64)
  - ASPPModule (L67-102): __init__ (L68-85), forward (L87-102)
  - LSTMModule (L105-125): __init__ (L106-114), forward (L116-125)

### infer/lib/uvr5_pack/lib_v5/model_param_init.py
* Functions: int_keys (L36-42)
* Classes:
  - ModelParameters (L45-69): __init__ (L46-69)

### infer/lib/uvr5_pack/lib_v5/nets.py
* Classes:
  - BaseASPPNet (L9-37): __init__ (L10-22), __call__ (L24-37)
  - CascadedASPPNet (L40-123): __init__ (L41-59), forward (L61-114), predict (L116-123)

### infer/lib/uvr5_pack/lib_v5/nets_123812KB.py
* Classes:
  - BaseASPPNet (L8-36): __init__ (L9-21), __call__ (L23-36)
  - CascadedASPPNet (L39-122): __init__ (L40-58), forward (L60-113), predict (L115-122)

### infer/lib/uvr5_pack/lib_v5/nets_123821KB.py
* Classes:
  - BaseASPPNet (L8-36): __init__ (L9-21), __call__ (L23-36)
  - CascadedASPPNet (L39-122): __init__ (L40-58), forward (L60-113), predict (L115-122)

### infer/lib/uvr5_pack/lib_v5/nets_33966KB.py
* Classes:
  - BaseASPPNet (L8-36): __init__ (L9-21), __call__ (L23-36)
  - CascadedASPPNet (L39-122): __init__ (L40-58), forward (L60-113), predict (L115-122)

### infer/lib/uvr5_pack/lib_v5/nets_537227KB.py
* Classes:
  - BaseASPPNet (L9-37): __init__ (L10-22), __call__ (L24-37)
  - CascadedASPPNet (L40-123): __init__ (L41-59), forward (L61-114), predict (L116-123)

### infer/lib/uvr5_pack/lib_v5/nets_537238KB.py
* Classes:
  - BaseASPPNet (L9-37): __init__ (L10-22), __call__ (L24-37)
  - CascadedASPPNet (L40-123): __init__ (L41-59), forward (L61-114), predict (L116-123)

### infer/lib/uvr5_pack/lib_v5/nets_61968KB.py
* Classes:
  - BaseASPPNet (L8-36): __init__ (L9-21), __call__ (L23-36)
  - CascadedASPPNet (L39-122): __init__ (L40-58), forward (L60-113), predict (L115-122)

### infer/lib/uvr5_pack/lib_v5/nets_new.py
* Classes:
  - BaseNet (L8-42): __init__ (L9-25), __call__ (L27-42)
  - CascadedNet (L45-133): __init__ (L46-76), forward (L78-114), predict_mask (L116-123), predict (L125-133)

### infer/lib/uvr5_pack/lib_v5/spec_utils.py
* Functions: crop_center (L12-27), wave_to_spectrogram (L30-51), wave_to_spectrogram_mt (L54-86), wave_to_spectrogram_mt.run_thread (L72-74), combine_spectrograms (L89-124), spectrogram_to_image (L127-148), reduce_vocal_aggressively (L151-159), mask_silence (L162-197), align_wave_head_and_tail (L200-203), cache_or_load (L206-292), spectrogram_to_wave (L295-316), spectrogram_to_wave_mt (L319-350), spectrogram_to_wave_mt.run_thread (L325-327), cmb_spectrogram_to_wave (L353-428), fft_lp_filter (L431-439), fft_hp_filter (L442-450), mirroring (L453-490), ensembling (L493-507), stft (L510-517), istft (L520-526), __main__ (L529-674)

### infer/lib/uvr5_pack/utils.py
* Functions: load_data (L8-12), make_padding (L15-22), inference (L25-99), inference._execute (L30-56), inference.preprocess (L58-62), _get_name_params (L102-121)

### infer/modules/ipex/__init__.py
* Functions: ipex_init (L12-190)

### infer/modules/ipex/attention.py
* Functions: torch_bmm (L9-78), scaled_dot_product_attention (L84-212), attention_init (L215-218)

### infer/modules/ipex/gradscaler.py
* Functions: _unscale_grads_ (L15-63), unscale_ (L66-113), update (L116-179), gradscaler_init (L182-187)

### infer/modules/ipex/hijacks.py
* Functions: _shutdown_workers (L46-77), return_null_context (L91-92), check_device (L95-100), return_xpu (L103-112), ipex_no_cuda (L115-118), ipex_autocast (L124-128), torch_cat (L134-144), interpolate (L150-180), linalg_solve (L186-193), ipex_hijacks (L196-365)
* Classes:
  - CondFunc (L9-40): __new__ (L10-29), __init__ (L31-34), __call__ (L36-40)
  - DummyDataParallel (L80-88): __new__ (L83-88)

### infer/modules/onnx/export.py
* Functions: export_onnx (L6-54)

### infer/modules/train/extract/extract_f0_print.py
* Functions: printt (L23-26), __main__ (L142-175)
* Classes:
  - FeatureInput (L33-139): __init__ (L34-42), compute_f0 (L44-93), coarse_f0 (L95-109), go (L111-139)

### infer/modules/train/extract/extract_f0_rmvpe.py
* Functions: printt (L27-30), __main__ (L105-128)
* Classes:
  - FeatureInput (L33-102): __init__ (L34-42), compute_f0 (L44-56), coarse_f0 (L58-72), go (L74-102)

### infer/modules/train/extract/extract_f0_rmvpe_dml.py
* Functions: printt (L25-28), __main__ (L103-126)
* Classes:
  - FeatureInput (L31-100): __init__ (L32-40), compute_f0 (L42-54), coarse_f0 (L56-70), go (L72-100)

### infer/modules/train/extract_feature_print.py
* Functions: forward_dml (L38-41), printt (L48-51), readwave (L66-77)

### infer/modules/train/preprocess.py
* Functions: println (L29-32), preprocess_trainset (L134-138), __main__ (L141-142)
* Classes:
  - PreProcess (L35-131): __init__ (L36-57), norm_write (L59-79), pipeline (L81-105), pipeline_mp (L107-109), pipeline_mp_inp_dir (L111-131)

### infer/modules/train/train.py
* Functions: main (L95-117), run (L120-296), train_and_evaluate (L299-635), __main__ (L638-640)
* Classes:
  - EpochRecorder (L82-92): __init__ (L83-84), record (L86-92)

### infer/modules/uvr5/mdxnet.py
* Functions: get_models (L78-87)
* Classes:
  - ConvTDFNetTrim (L15-75): __init__ (L16-39), stft (L41-56), istft (L58-75)
  - Predictor (L90-238): __init__ (L91-107), demix (L109-141), demix_base (L143-197), prediction (L199-238)
  - MDXNetDereverb (L241-256): __init__ (L242-253), _path_audio_ (L255-256)

### infer/modules/uvr5/modules.py
* Functions: uvr (L17-108)

### infer/modules/uvr5/vr.py
* Classes:
  - AudioPre (L18-195): __init__ (L19-42), _path_audio_ (L44-195)
  - AudioPreDeEcho (L198-368): __init__ (L199-223), _path_audio_ (L225-368)

### infer/modules/vc/__init__.py

### infer/modules/vc/modules.py
* Classes:
  - VC (L22-304): __init__ (L23-34), get_vc (L36-144), vc_single (L146-225), vc_multi (L227-304)

### infer/modules/vc/pipeline.py
* Functions: cache_harvest_f0 (L30-40), change_rms (L43-62)
* Classes:
  - Pipeline (L65-457): __init__ (L66-82), get_f0 (L84-184), vc (L186-279), pipeline (L281-457)

### infer/modules/vc/utils.py
* Functions: get_index_path_from_model (L6-19), load_hubert (L22-33)

### infer-web.py
* Functions: forward_dml (L59-62), lookup_indices (L145-150), change_choices (L161-174), clean (L177-178), export_onnx (L181-184), if_done (L194-200), if_done_multi (L203-215), preprocess_dataset (L218-254), extract_f0_feature (L258-395), get_pretrained_models (L398-430), change_sr2 (L433-436), change_version19 (L439-452), change_f0 (L455-461), click_train (L465-612), train_index (L616-711), train1key (L715-778), train1key.get_info_str (L737-739), change_info_ (L782-795), change_f0_method (L801-806)
* Classes:
  - ToolButton (L123-130): __init__ (L126-127), get_block_name (L129-130)

### tools/app.py

### tools/calc_rvc_model_similarity.py
* Functions: cal_cross_attn (L13-29), model_hash (L32-43), eval (L46-53), main (L56-90), __main__ (L93-96)

### tools/download_models.py
* Constants: RVC_DOWNLOAD_LINK, BASE_DIR
* Functions: dl_model (L10-16), __main__ (L19-79)

### tools/export_onnx.py
* Functions: __main__ (L4-54)

### tools/infer/infer-pm-index256.py
* Functions: get_f0 (L88-120)

### tools/infer/train-index-v2.py

### tools/infer/train-index.py

### tools/infer/trans_weights.py

### tools/infer_batch_rvc.py
* Functions: arg_parse (L19-38), main (L41-68), __main__ (L71-72)

### tools/infer_cli.py
* Functions: arg_parse (L19-38), main (L41-63), __main__ (L66-67)

### tools/onnx_inference_demo.py

### tools/rvc_for_realtime.py
* Functions: printt (L38-42)
* Classes:
  - RVC (L47-445): __init__ (L48-193), change_key (L195-196), change_index_rate (L198-203), get_f0_post (L205-216), get_f0 (L218-287), get_f0_crepe (L289-311), get_f0_rmvpe (L313-326), get_f0_fcpe (L328-345), infer (L347-445)

### tools/torchgate/__init__.py

### tools/torchgate/torchgate.py
* Classes:
  - TorchGate (L8-280): __init__ (L33-72), _generate_mask_smoothing_filter (L75-125), _stationary_mask (L128-175), _nonstationary_mask (L178-208), forward (L210-280)

### tools/torchgate/utils.py
* Functions: amp_to_db (L6-25), temperature_sigmoid (L29-41), linspace (L45-70)
